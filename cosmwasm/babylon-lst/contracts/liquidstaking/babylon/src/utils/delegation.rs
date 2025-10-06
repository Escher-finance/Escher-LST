use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{
    Addr, AnyMsg, Attribute, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, Event,
    QuerierWrapper, StdResult, Storage, SubMsg, Timestamp, Uint128, to_json_binary,
};
use prost::Message;
use unionlabs_primitives::{Bytes, H256};

use super::{
    authz::get_authz_ucs03_transfer,
    batch::{Batch, BatchStatus},
    calc::{calculate_exchange_rate, calculate_fee_from_reward},
};
use crate::{
    ContractError,
    event::{SubmitBatchEvent, UnbondEventsFromAtts, UnstakeRequestEvent},
    execute::StakerUndelegation,
    msg::{
        BondData, DelegationDiff, InjectData, LiquidityState, MintTokensPayload, RemoteBondData,
        ValidatorDelegation,
    },
    proto,
    state::{
        BurnQueue, MintQueue, PARAMETERS, PENDING_BATCH_ID, Parameters, QUOTE_TOKEN,
        REWARD_BALANCE, STATE, SUPPLY_QUEUE, SupplyQueue, UNBOND_RECIPIENT_IBC_CHANNEL,
        UnbondRecord, VALIDATORS_REGISTRY, Validator, ValidatorsRegistry, WITHDRAW_REWARD_QUEUE,
        increment_tokens, unbond_record,
    },
    utils::{
        batch::batches,
        calc::{self, check_slippage_with_min_mint_amount},
        delegation, token,
    },
};

pub const DEFAULT_TIMEOUT_TIMESTAMP_OFFSET: u64 = 900;

/// get total delegated token value from validators in native token
/// # Result
/// Will return `StdResult` of total delegated amount in Uint128
/// # Errors
/// Will return `StdError`
pub fn get_actual_total_delegated(
    querier: QuerierWrapper,
    delegator: String,
    denom: &str,
    validators: &[String],
) -> StdResult<Uint128> {
    let delegations_resp = querier.query_all_delegations(delegator)?;

    Ok(delegations_resp
        .into_iter()
        .filter(|d| {
            d.amount.denom == denom
                && !d.amount.amount.is_zero()
                && validators.contains(&d.validator)
        })
        .map(|d| d.amount.amount)
        .sum())
}

/// get unclaimed reward from validators in native token
/// # Result
/// Will return `StdResult` of total unclaimed erward amount in Uint128
/// # Errors
/// Will return `StdError`
pub fn get_unclaimed_reward(
    querier: QuerierWrapper,
    delegator: String,
    denom: &str,
    validators: &[String],
) -> StdResult<Uint128> {
    let mut total_rewards = Uint128::new(0);
    let result = querier.query_delegation_total_rewards(delegator)?;

    for delegator_reward in result.rewards {
        if validators.contains(&delegator_reward.validator_address) {
            for reward in delegator_reward.reward {
                if reward.denom == denom {
                    let reward_val = calc::to_uint128(reward.amount.to_uint_floor())?;
                    total_rewards += reward_val;
                }
            }
        }
    }

    Ok(total_rewards)
}

/// get mock total reward for test purpose only
#[must_use]
pub fn get_mock_total_reward(total_bond_amount: Uint128) -> Uint128 {
    let ratio = Decimal::from_ratio(Uint128::new(1000), Uint128::new(1005));
    calc::calculate_staking_token_from_rate(total_bond_amount, ratio)
}

/// calculate delegated amount based on ratio
#[must_use]
pub fn calculate_delegated_amount(amount: Uint128, ratio: Decimal) -> Uint128 {
    (ratio * Decimal::from_ratio(amount, Uint128::one())).to_uint_floor()
}

#[must_use]
pub fn get_undelegate_msgs(
    delegator: String,
    undelegate_amount: Uint128,
    coin_denom: String,
    validator_delegation_ratio: HashMap<String, Decimal>,
) -> (Uint128, Vec<CosmosMsg>, Vec<Attribute>) {
    let mut msgs: Vec<CosmosMsg> = vec![];

    let mut atts = vec![];

    let mut total_undelegate_amount = Uint128::zero();

    let undelegate_amount_dec: Decimal = Decimal::from_ratio(undelegate_amount, Uint128::one());

    for (validator, ratio) in validator_delegation_ratio {
        let undelegate_amount_for_validator = (undelegate_amount_dec * ratio).to_uint_floor();

        if undelegate_amount_for_validator.is_zero() {
            continue;
        }

        let undelegate_staking_msg = get_babylon_undelegate_cosmos_msg(
            delegator.clone(),
            validator.clone(),
            undelegate_amount_for_validator.to_string(),
            coin_denom.clone(),
        );
        msgs.push(undelegate_staking_msg);

        atts.push(Attribute {
            key: validator,
            value: undelegate_amount_for_validator.to_string(),
        });

        total_undelegate_amount += undelegate_amount_for_validator;
    }

    (total_undelegate_amount, msgs, atts)
}

/// get map of validator address and delegation amount with total bond amount
/// # Result
/// Will return `StdResult` of total unclaimed erward amount in Uint128
/// # Errors
/// Will return `StdError`
pub fn get_validator_delegation_map_with_total_bond(
    deps: Deps,
    delegator: &str,
    validators: Vec<Validator>,
) -> Result<(HashMap<String, Uint128>, Uint128), ContractError> {
    let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    let mut total_delegated_amount = Uint128::from(0u32);

    for validator in validators {
        let validator_bond = deps
            .querier
            .query_delegation(delegator, validator.address.clone())?;

        let delegation_amount = match validator_bond {
            Some(delegation) => delegation.amount.amount,
            None => Uint128::from(0u32),
        };
        validator_delegation_map.insert(validator.address.clone(), delegation_amount);
        total_delegated_amount += delegation_amount;
    }

    Ok((validator_delegation_map, total_delegated_amount))
}

#[must_use]
pub fn get_validator_delegation_map_base_on_weight(
    validators: Vec<Validator>,
    total_delegated_amount: Uint128,
) -> HashMap<String, Uint128> {
    let total_weight = validators
        .iter()
        .map(|v| v.weight)
        .reduce(|a, b| a + b)
        .unwrap_or(0);

    let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    let mut total_delegation_amount = Uint128::zero();
    let mut first_validator = String::new();
    for validator in validators {
        let ratio = Decimal::from_ratio(validator.weight, total_weight);

        let delegation_amount = calculate_delegated_amount(total_delegated_amount, ratio);
        total_delegation_amount += delegation_amount;

        let validator_address = validator.address.clone();
        if first_validator.is_empty() {
            first_validator.clone_from(&validator_address);
        }

        correct_validator_delegation_map.insert(validator_address, delegation_amount);
    }

    let remaining_amount = total_delegated_amount - total_delegation_amount;
    if !remaining_amount.is_zero() {
        correct_validator_delegation_map
            .entry(first_validator)
            .and_modify(|amount| *amount += remaining_amount);
    }

    correct_validator_delegation_map
}

#[must_use]
pub fn get_surplus_deficit_validators(
    validator_delegation_map: HashMap<String, Uint128>,
    correct_validator_delegation_map: HashMap<String, Uint128>,
) -> (Vec<ValidatorDelegation>, Vec<ValidatorDelegation>) {
    let mut surplus_validators: Vec<ValidatorDelegation> = vec![];
    let mut deficient_validators: Vec<ValidatorDelegation> = vec![];
    for (key, previous_amount) in validator_delegation_map.clone().iter_mut() {
        // check if old validator key exists on new validators map
        if !correct_validator_delegation_map.contains_key(key) {
            // because old validator not exists on new one that means the previous validator
            // need to be restaked fully so it is surplus
            surplus_validators.push(ValidatorDelegation {
                address: key.to_string(),
                delegation_diff: DelegationDiff::Surplus,
                diff_amount: *previous_amount,
            })
        };
    }

    for (new_validator_key, correct_amount) in correct_validator_delegation_map.clone().iter_mut() {
        // check if previous validator exists
        match validator_delegation_map.get(new_validator_key) {
            Some(previous_amount) => {
                if *previous_amount > *correct_amount {
                    surplus_validators.push(ValidatorDelegation {
                        address: new_validator_key.to_string(),
                        delegation_diff: DelegationDiff::Surplus,
                        diff_amount: *previous_amount - *correct_amount,
                    })
                } else {
                    deficient_validators.push(ValidatorDelegation {
                        address: new_validator_key.to_string(),
                        delegation_diff: DelegationDiff::Deficit,
                        diff_amount: *correct_amount - *previous_amount,
                    })
                }
            }
            None => deficient_validators.push(ValidatorDelegation {
                address: new_validator_key.to_string(),
                delegation_diff: DelegationDiff::Deficit,
                diff_amount: *correct_amount,
            }),
        }
    }

    (surplus_validators, deficient_validators)
}

#[must_use]
pub fn get_restaking_msgs(
    delegator: String,
    mut surplus_validators: Vec<ValidatorDelegation>,
    mut deficient_validators: Vec<ValidatorDelegation>,
    denom: String,
) -> Vec<CosmosMsg> {
    let mut msgs = Vec::new();

    // Sort surplus and deficient validators for deterministic behavior
    surplus_validators.sort_by_key(|v| v.address.clone());
    deficient_validators.sort_by_key(|v| v.address.clone());

    for surplus_validator in surplus_validators.iter_mut() {
        while surplus_validator.diff_amount > Uint128::zero() {
            if let Some(deficient_validator) = deficient_validators
                .iter_mut()
                .find(|v| v.diff_amount > Uint128::zero())
            {
                // Calculate the amount to redelegate
                let redelegate_amount = surplus_validator
                    .diff_amount
                    .min(deficient_validator.diff_amount);

                // Skip invalid redelegate amounts
                if redelegate_amount.is_zero() {
                    continue;
                }

                // if test mode, we need to use the StakingMsg::Redelegate so it is readable and can be asserted
                // otherwise we use the MsgWrappedBeginRedelegate that will be sent to babylon
                if cfg!(test) {
                    msgs.push(CosmosMsg::Staking(cosmwasm_std::StakingMsg::Redelegate {
                        src_validator: surplus_validator.address.clone(),
                        dst_validator: deficient_validator.address.clone(),
                        amount: Coin {
                            denom: denom.clone(),
                            amount: redelegate_amount,
                        },
                    }));
                } else {
                    msgs.push(get_babylon_redelegate_cosmos_msg(
                        delegator.clone(),
                        surplus_validator.address.to_string(),
                        deficient_validator.address.clone(),
                        redelegate_amount.to_string(),
                        denom.clone(),
                    ));
                }

                // Update the surplus and deficit amounts
                surplus_validator.diff_amount -= redelegate_amount;
                deficient_validator.diff_amount -= redelegate_amount;
            } else {
                // No more deficient validators to process
                break;
            }
        }
    }

    msgs
}

pub fn get_delegate_to_validator_msgs(
    delegator: String,
    delegate_amount: Uint128,
    coin_denom: String,
    validators: Vec<Validator>,
) -> Vec<CosmosMsg> {
    let total_weight = validators
        .iter()
        .map(|v| v.weight)
        .reduce(|a, b| a + b)
        .unwrap_or(1);

    let mut total_delegated: Uint128 = Uint128::from(0u32);

    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut first_validator: String = String::new();

    for validator in validators {
        let ratio =
            Decimal::from_ratio(Uint128::from(validator.weight), Uint128::from(total_weight));

        let delegate_amount = calculate_delegated_amount(delegate_amount, ratio);

        if delegate_amount == Uint128::zero() {
            continue;
        }

        total_delegated += delegate_amount;

        let delegate_msg = get_babylon_delegate_cosmos_msg(
            delegator.clone(),
            validator.address.clone(),
            delegate_amount.to_string(),
            coin_denom.clone(),
        );

        msgs.push(delegate_msg);

        if first_validator.is_empty() {
            first_validator = validator.address.clone();
        }
    }

    // calculate remaining

    let remaining_amount = delegate_amount - total_delegated;
    if !remaining_amount.is_zero() {
        let delegate_msg = get_babylon_delegate_cosmos_msg(
            delegator.clone(),
            first_validator.clone(),
            remaining_amount.to_string(),
            coin_denom,
        );

        msgs.push(delegate_msg);
    }
    msgs
}

/// get cosmos msg to restake from surplus validator to deficient validator
/// # Result
/// Will return `StdResult` of total unclaimed erward amount in Uint128
/// # Errors
/// Will return `StdError`
pub fn adjust_validators_delegation(
    deps: DepsMut,
    delegator: Addr,
    prev_validators: Vec<Validator>,
    validators: Vec<Validator>,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let denom = params.underlying_coin_denom;

    let (validator_delegation_map, total_delegated_amount) =
        get_validator_delegation_map_with_total_bond(
            deps.as_ref(),
            &delegator.to_string(),
            prev_validators,
        )?;

    let correct_validator_delegation_map =
        get_validator_delegation_map_base_on_weight(validators, total_delegated_amount);

    let (surplus_validators, deficient_validators) =
        get_surplus_deficit_validators(validator_delegation_map, correct_validator_delegation_map);

    let msgs: Vec<CosmosMsg> = get_restaking_msgs(
        delegator.to_string(),
        surplus_validators,
        deficient_validators,
        denom,
    );

    Ok(msgs)
}

/// Process bond call to mint liquid staking token, delegate/stake base on the bond amount and exchange rate
/// # Result
/// Will return `StdResult` of Vector of `CosmosMsg`, Vector of `SubMsg` and `BondData`
/// # Errors
/// Will return `ContractError`
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn process_bond(
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
    sender: String,
    staker: String,
    delegator: Addr,
    amount: Uint128,
    bond_time: u64,
    params: Parameters,
    validators_reg: ValidatorsRegistry,
    salt: String,
    channel_id: Option<u32>,
    block_height: u64,
    recipient: Option<String>,
    recipient_channel_id: Option<u32>,
    on_chain_recipient: bool,
    transfer_fee: Option<Uint128>,
) -> Result<(Vec<CosmosMsg>, Vec<SubMsg>, BondData), ContractError> {
    if amount < params.min_bond {
        return Err(ContractError::BondAmountTooLow {});
    }

    let coin_denom = params.underlying_coin_denom.clone();
    let msgs = delegation::get_delegate_to_validator_msgs(
        delegator.to_string(),
        amount,
        coin_denom.clone(),
        validators_reg.validators.clone(),
    );

    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    // logic to mint token and update the supply and total_bond_amount
    let mut state = STATE.load(storage)?;

    let delegated_amount = get_actual_total_delegated(
        querier,
        delegator.to_string(),
        &coin_denom,
        &validators_list,
    )?;

    let total_bond_amount: Uint128;
    let reward_balance: Uint128;
    let unclaimed_reward: Uint128;
    if cfg!(test) {
        total_bond_amount = get_mock_total_reward(state.total_bond_amount);
        reward_balance = Uint128::zero();
        unclaimed_reward = Uint128::zero();
    } else {
        state.total_delegated_amount = delegated_amount;
        unclaimed_reward = get_unclaimed_reward(
            querier,
            delegator.to_string(),
            &coin_denom,
            &validators_list,
        )?;

        reward_balance = calc::normalize_reward_balance(storage, block_height, unclaimed_reward)?;

        let reward = reward_balance + unclaimed_reward;
        let fee = calculate_fee_from_reward(reward, params.fee_rate);
        total_bond_amount = delegated_amount + reward - fee;
    }

    let mut supply_queue: SupplyQueue = SUPPLY_QUEUE.load(storage)?;
    calc::normalize_supply_queue(&mut supply_queue, block_height);
    let exchange_rate = if total_bond_amount == Uint128::zero() {
        Decimal::one()
    } else {
        calc::calculate_exchange_rate(total_bond_amount, state.total_supply, &supply_queue)
    };

    if exchange_rate < Decimal::one() {
        return Err(ContractError::InvalidExchangeRate {});
    }

    let mint_amount = calc::calculate_staking_token_from_rate(amount, exchange_rate);

    // after update exchange rate we update the state
    state.bond_counter += 1;
    state.total_bond_amount = total_bond_amount + amount;
    state.total_supply += mint_amount;
    state.total_delegated_amount += amount;
    state.last_bond_time = bond_time;
    state.exchange_rate = exchange_rate;

    STATE.save(storage, &state)?;

    let mut sub_msgs: Vec<SubMsg> = vec![];
    let payload = MintTokensPayload {
        sender: sender.clone(),
        staker: staker.clone(),
        amount: mint_amount,
        salt,
        channel_id,
        recipient,
        recipient_channel_id,
        transfer_fee,
    };
    let payload_bin = to_json_binary(&payload)?;

    supply_queue.mint.push(MintQueue {
        block: block_height,
        amount: mint_amount,
    });
    SUPPLY_QUEUE.save(storage, &supply_queue)?;

    if !cfg!(test) {
        let minted_token_recipient =
            if !on_chain_recipient && (channel_id.is_some() || recipient_channel_id.is_some()) {
                params.transfer_handler
            } else {
                delegator.to_string()
            };

        // Start to mint according to staked token only if it is not test
        let sub_msg: SubMsg = token::get_staked_token_submsg(
            minted_token_recipient,
            mint_amount,
            params.liquidstaking_denom.clone(),
            payload_bin,
            &params.cw20_address,
        );
        sub_msgs.push(sub_msg);
    }

    Ok((
        msgs,
        sub_msgs,
        BondData {
            mint_amount,
            delegated_amount: state.total_delegated_amount,
            total_bond_amount: state.total_bond_amount,
            exchange_rate,
            total_supply: state.total_supply,
            reward_balance,
            unclaimed_reward,
        },
    ))
}

pub fn query_liquidity(
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
    delegator: String,
    coin_denom: String,
    validators_list: Vec<String>,
    block_height: u64,
    fee_rate: Decimal,
    supply_queue: &mut SupplyQueue,
) -> Result<LiquidityState, ContractError> {
    let mut state = STATE.load(storage)?;

    let delegated_amount =
        get_actual_total_delegated(querier, delegator.clone(), &coin_denom, &validators_list)?;

    state.total_delegated_amount = delegated_amount;
    let unclaimed_reward = get_unclaimed_reward(querier, delegator, &coin_denom, &validators_list)?;

    let reward_balance =
        calc::normalize_reward_balance(storage, block_height, unclaimed_reward).unwrap();

    let reward = reward_balance + unclaimed_reward;

    let fee: Uint128 = calculate_fee_from_reward(reward, fee_rate);
    let assets = delegated_amount + reward - fee;

    calc::normalize_supply_queue(supply_queue, block_height);
    let exchange_rate = if assets == Uint128::zero() {
        Decimal::one()
    } else {
        calc::calculate_exchange_rate(assets, state.total_supply, supply_queue)
    };

    if exchange_rate < Decimal::one() {
        return Err(ContractError::InvalidExchangeRate {});
    }

    Ok(LiquidityState {
        assets,
        delegated: delegated_amount,
        reward_balance,
        unclaimed_reward,
        exchange_rate,
    })
}

/// Process delegate call to mint liquid staking token, delegate/stake base on the bond amount and exchange rate
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
pub fn delegate(
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
    env: Env,
    amount: Uint128,
    min_mint_amount: Uint128,
) -> Result<(Vec<CosmosMsg>, RemoteBondData), ContractError> {
    let params = PARAMETERS.load(storage)?;

    let validators_reg = VALIDATORS_REGISTRY.load(storage)?;

    let msgs = delegation::get_delegate_to_validator_msgs(
        env.contract.address.to_string(),
        amount,
        params.underlying_coin_denom.clone(),
        validators_reg.validators.clone(),
    );

    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    let mut supply_queue: SupplyQueue = SUPPLY_QUEUE.load(storage)?;

    let liquidity = query_liquidity(
        storage,
        querier,
        env.contract.address.to_string(),
        params.underlying_coin_denom.clone(),
        validators_list,
        env.block.height,
        params.fee_rate,
        &mut supply_queue,
    )?;

    let mint_amount = calc::calculate_staking_token_from_rate(amount, liquidity.exchange_rate);

    check_slippage_with_min_mint_amount(min_mint_amount, mint_amount, Decimal::percent(1))?;

    supply_queue.mint.push(MintQueue {
        block: env.block.height,
        amount: mint_amount,
    });
    SUPPLY_QUEUE.save(storage, &supply_queue)?;

    // logic to mint token and update the supply and total_bond_amount
    let mut state = STATE.load(storage)?;
    // after update exchange rate we update the state
    state.bond_counter += 1;
    state.total_bond_amount = liquidity.assets + amount;
    state.total_supply += mint_amount;
    state.total_delegated_amount = liquidity.delegated + amount;
    state.last_bond_time = env.block.time.nanos();
    state.exchange_rate = liquidity.exchange_rate;

    STATE.save(storage, &state)?;

    Ok((
        msgs,
        RemoteBondData {
            denom: params.underlying_coin_denom.clone(),
            bond_amount: amount,
            mint_amount,
            delegated_amount: state.total_delegated_amount,
            total_bond_amount: state.total_bond_amount,
            exchange_rate: liquidity.exchange_rate,
            total_supply: state.total_supply,
            reward_balance: liquidity.reward_balance,
            unclaimed_reward: liquidity.unclaimed_reward,
            cw20_address: params.cw20_address.to_string(),
        },
    ))
}

/// Process unstake requests from batch that will burn liquid staking token, undelegate some amount from validator according to exchange rate and create UnbondRecord
/// 1. Undelegate to validators
/// 2. Set current batch status to submitted
/// 3. Create new `SubmitBatchEvent`
/// 4. Create new pending batch
/// # Result
/// Will return result of `cosmwasm_std::Response` of `CosmosMsg` Vector and Event Vector
/// # Errors
/// Will return contract error
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn submit_pending_batch(
    deps: DepsMut,
    block_height: u64,
    time: Timestamp,
    sender: Addr,
    delegator: Addr,
    batch: &mut Batch,
    params: Parameters,
    validators_reg: &ValidatorsRegistry,
) -> Result<(Vec<CosmosMsg>, Vec<Event>), ContractError> {
    let coin_denom = params.underlying_coin_denom;

    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    let mut state = STATE.load(deps.storage)?;

    let (validator_delegation_map, delegated_amount) =
        get_validator_delegation_map_with_total_bond(
            deps.as_ref(),
            delegator.as_ref(),
            validators_reg.validators.clone(),
        )?;

    state.total_delegated_amount = delegated_amount;
    // query the total reward from this contract
    let unclaimed_reward = get_unclaimed_reward(
        deps.querier,
        delegator.to_string(),
        &coin_denom,
        &validators_list,
    )?;

    let reward_balance = REWARD_BALANCE.load(deps.storage)?;

    let mut supply_queue: SupplyQueue = SUPPLY_QUEUE.load(deps.storage)?;

    // if there is no unstake/liquid stake amount in batch then no need to normalize reward balance
    let new_balance = if batch.total_liquid_stake.is_zero() {
        let withdraw_reward_queue = WITHDRAW_REWARD_QUEUE.load(deps.storage)?;
        let (new_balance, new_queue) = calc::normalize_withdraw_reward_queue(
            block_height,
            reward_balance,
            withdraw_reward_queue,
            supply_queue.epoch_period,
        );
        REWARD_BALANCE.save(deps.storage, &new_balance)?;
        WITHDRAW_REWARD_QUEUE.save(deps.storage, &new_queue)?;
        new_balance
    } else {
        calc::normalize_reward_balance(deps.storage, block_height, unclaimed_reward)?
    };

    let reward: Uint128 = unclaimed_reward + new_balance;

    let fee = calculate_fee_from_reward(reward, params.fee_rate);
    let total_bond_amount = delegated_amount + reward - fee;

    if total_bond_amount.is_zero() || state.total_supply.is_zero() {
        return Err(ContractError::ZeroSupplyOrDelegatedAmount {});
    }

    calc::normalize_supply_queue(&mut supply_queue, block_height);
    let current_exchange_rate = if total_bond_amount == Uint128::zero() {
        Decimal::one()
    } else {
        calc::calculate_exchange_rate(total_bond_amount, state.total_supply, &supply_queue)
    };

    // calculate how much native token undelegated amount from staked token amount base on current exchange rate
    let undelegate_amount: Uint128 = calc::calculate_native_token_from_staking_token(
        batch.total_liquid_stake,
        current_exchange_rate,
    );

    let mut msgs: Vec<CosmosMsg> = vec![];
    if delegated_amount < undelegate_amount {
        return Err(ContractError::NotEnoughAvailableFund {}); // this error only happen on development or sole staker and if process rewards not happen yet
    }

    let mut validators_delegation_ratio: HashMap<String, Decimal> = HashMap::new();

    for (validator, amount) in validator_delegation_map {
        let ratio = Decimal::from_ratio(amount, delegated_amount);
        validators_delegation_ratio.insert(validator, ratio);
    }

    let (total_undelegate_amount, undelegate_msgs, atts) = get_undelegate_msgs(
        delegator.to_string(),
        undelegate_amount,
        coin_denom.clone(),
        validators_delegation_ratio,
    );
    msgs.extend(undelegate_msgs.clone());

    let mut events = UnbondEventsFromAtts(atts, batch.id, time);

    let burn_msg = token::burn_token(batch.total_liquid_stake, params.cw20_address.to_string());
    msgs.push(burn_msg);

    supply_queue.burn.push(BurnQueue {
        block: block_height,
        amount: total_undelegate_amount,
    });
    SUPPLY_QUEUE.save(deps.storage, &supply_queue)?;

    // // update total bond, supply and exchange rate here
    state.total_bond_amount = total_bond_amount - total_undelegate_amount;
    state.total_supply -= batch.total_liquid_stake;
    state.total_delegated_amount = delegated_amount - total_undelegate_amount;
    state.exchange_rate =
        calculate_exchange_rate(state.total_bond_amount, state.total_supply, &supply_queue);
    STATE.save(deps.storage, &state)?;

    batch.expected_native_unstaked = Some(total_undelegate_amount);
    // no unbond record in this batch or no liquid stake amount
    // so we can release the batch and assume it as completed/done
    if batch.total_liquid_stake.is_zero() {
        batch.update_status(BatchStatus::Released, None);
    } else {
        let next_action_time = time.seconds() + params.unbonding_time;
        batch.update_status(BatchStatus::Submitted, Some(next_action_time));
    }
    batches().save(deps.storage, batch.id, batch)?;

    let ev = SubmitBatchEvent(
        batch.id,
        sender.to_string(),
        batch.total_liquid_stake,
        total_undelegate_amount,
        delegated_amount,
        total_bond_amount,
        state.total_supply,
        current_exchange_rate,
        time,
        coin_denom,
        new_balance,
        unclaimed_reward,
    );

    events.push(ev);

    // Create new pending batch
    let new_pending_batch = Batch::new(
        batch.id + 1,
        Uint128::zero(),
        time.seconds() + params.batch_period,
    );
    batches().save(deps.storage, new_pending_batch.id, &new_pending_batch)?;
    PENDING_BATCH_ID.save(deps.storage, &new_pending_batch.id)?;

    Ok((msgs, events))
}

/// Create unbond requests that will create unbond record and put in pending batch
/// 1. Increase total liquid stake amount in pending batch
/// 2. Create unbond record and save in pending batch
/// 3. Create `UnstakeRequest` event
/// # Result
/// Will return result of `cosmwasm_std::Response` of `Event`
/// # Errors
/// Will return contract error
#[allow(clippy::too_many_arguments)]
pub fn unstake_request_in_batch(
    env: &Env,
    storage: &mut dyn Storage,
    sender: String,
    staker: String,
    unstake_amount: Uint128,
    channel_id: Option<u32>,
    recipient: Option<String>,
    recipient_channel_id: Option<u32>,
    recipient_ibc_channel_id: Option<String>,
) -> Result<Event, ContractError> {
    let params = PARAMETERS.load(storage)?;

    if unstake_amount < params.min_unbond {
        return Err(ContractError::UnbondAmountTooLow {});
    }

    let pending_batch_id = PENDING_BATCH_ID.load(storage)?;
    let mut pending_batch = batches().load(storage, pending_batch_id)?;

    // update total unstaked liquid stake amount in batch and increase unbond records count
    pending_batch.total_liquid_stake += unstake_amount;
    pending_batch.unbond_records_count += 1;
    batches().save(storage, pending_batch_id, &pending_batch)?;

    // adjust reward balance
    let reward_balance = REWARD_BALANCE.load(storage)?;
    let withdraw_reward_queue = WITHDRAW_REWARD_QUEUE.load(storage)?;
    let supply = SUPPLY_QUEUE.load(storage)?;
    let (new_balance, new_queue) = calc::normalize_withdraw_reward_queue(
        env.block.height,
        reward_balance,
        withdraw_reward_queue,
        supply.epoch_period,
    );
    REWARD_BALANCE.save(storage, &new_balance)?;
    WITHDRAW_REWARD_QUEUE.save(storage, &new_queue)?;

    let id: u64 = increment_tokens(storage)?;
    let record = UnbondRecord {
        id,
        batch_id: pending_batch.id,
        height: env.block.height,
        channel_id,
        sender: sender.clone(),
        staker: staker.clone(),
        amount: unstake_amount,
        released_height: 0,
        released: false,
        recipient: recipient.clone(),
        recipient_channel_id,
    };
    unbond_record().save(storage, id, &record)?;

    UNBOND_RECIPIENT_IBC_CHANNEL.save(storage, id, &recipient_ibc_channel_id)?;

    let event = UnstakeRequestEvent(
        sender,
        staker,
        channel_id,
        unstake_amount,
        record.id,
        pending_batch.id,
        env.block.time,
        recipient,
        recipient_channel_id,
        reward_balance,
        recipient_ibc_channel_id,
    );

    Ok(event)
}

/// get auth cosmos msg to transfer undelegated fund to recipient via ucs03
/// # Result
/// Will return result of CosmosMsg
/// # Errors
/// Will return contract error
#[allow(clippy::too_many_arguments)]
pub fn get_unbonding_ucs03_transfer_cosmos_msg(
    storage: &mut dyn Storage,
    lst_contract: &Addr,
    recipient: String,
    channel_id: u32,
    time: Timestamp,
    ucs03_relay_contract: &str,
    undelegate_amount: Uint128,
    transfer_fee: Uint128,
    denom: &str,
    salt: &str,
) -> Result<CosmosMsg, ContractError> {
    let total_amount = undelegate_amount + transfer_fee;

    // for the amount
    let funds = vec![Coin {
        denom: denom.to_string(),
        amount: total_amount,
    }];

    let params = PARAMETERS.load(storage)?;
    // get quote token of native base denom (muno) on specific channel id
    let quote_token = QUOTE_TOKEN.load(storage, channel_id)?;

    let quote_token_string = quote_token.quote_token.clone();

    let Ok(recipient_address) = Bytes::from_str(recipient.as_str()) else {
        return Err(ContractError::InvalidAddress {
            kind: "recipient".into(),
            address: recipient,
            reason: "address must be in hex and starts with 0x".to_string(),
        });
    };
    let Ok(quote_token) = Bytes::from_str(quote_token_string.as_str()) else {
        return Err(ContractError::InvalidAddress {
            kind: "quote_token".into(),
            address: quote_token_string,
            reason: "address must be in hex and starts with 0x".to_string(),
        });
    };

    let authz_ucs03_msg = get_authz_ucs03_transfer(
        params.cw20_address.to_string(),
        params.transfer_handler,  // granter
        lst_contract.to_string(), // grantee
        time,
        ucs03_relay_contract.into(),
        channel_id,
        recipient_address,
        denom.to_string(),
        total_amount,
        quote_token,
        undelegate_amount,
        &funds,
        H256::from_str(salt)?,
    )?;

    Ok(authz_ucs03_msg)
}

/// Errors:
/// - Returns serialization errors when querying rewards or reading storage.
pub fn get_actual_total_reward(
    storage: &dyn Storage,
    querier: QuerierWrapper,
    delegator: String,
    denom: &str,
    validators: &[String],
) -> StdResult<Uint128> {
    let unclaimed_reward = get_unclaimed_reward(querier, delegator, denom, validators)?;
    let reward_balance = REWARD_BALANCE.load(storage)?;
    Ok(unclaimed_reward + reward_balance)
}

#[must_use]
pub fn get_babylon_delegate_cosmos_msg(
    delegator_address: String,
    validator_address: String,
    amount: String,
    denom: String,
) -> CosmosMsg {
    let delegate_msg = proto::cosmos::staking::v1beta1::MsgDelegate {
        delegator_address,
        validator_address,
        amount: Some(proto::cosmos::base::v1beta1::Coin { denom, amount }),
    };

    let staking_msg: proto::babylon::epoching::v1::MsgWrappedDelegate =
        proto::babylon::epoching::v1::MsgWrappedDelegate {
            msg: Some(delegate_msg),
        };

    CosmosMsg::Any(AnyMsg {
        type_url: "/babylon.epoching.v1.MsgWrappedDelegate".to_string(),
        value: Binary::from(staking_msg.encode_to_vec()),
    })
}

#[must_use]
pub fn get_babylon_undelegate_cosmos_msg(
    delegator_address: String,
    validator_address: String,
    amount: String,
    denom: String,
) -> CosmosMsg {
    let undelegate_msg: proto::cosmos::staking::v1beta1::MsgUndelegate =
        proto::cosmos::staking::v1beta1::MsgUndelegate {
            delegator_address,
            validator_address,
            amount: Some(proto::cosmos::base::v1beta1::Coin { denom, amount }),
        };

    let wrapped_msg: proto::babylon::epoching::v1::MsgWrappedUndelegate =
        proto::babylon::epoching::v1::MsgWrappedUndelegate {
            msg: Some(undelegate_msg),
        };

    CosmosMsg::Any(AnyMsg {
        type_url: "/babylon.epoching.v1.MsgWrappedUndelegate".to_string(),
        value: Binary::from(wrapped_msg.encode_to_vec()),
    })
}

pub fn get_babylon_redelegate_cosmos_msg(
    delegator_address: String,
    validator_src_address: String,
    validator_dst_address: String,
    amount: String,
    denom: String,
) -> CosmosMsg {
    let redelegate_msg = proto::cosmos::staking::v1beta1::MsgBeginRedelegate {
        delegator_address,
        validator_src_address,
        validator_dst_address,
        amount: Some(proto::cosmos::base::v1beta1::Coin {
            denom: denom.clone(),
            amount,
        }),
    };

    let restaking_msg: proto::babylon::epoching::v1::MsgWrappedBeginRedelegate =
        proto::babylon::epoching::v1::MsgWrappedBeginRedelegate {
            msg: Some(redelegate_msg),
        };

    CosmosMsg::Any(AnyMsg {
        type_url: "/babylon.epoching.v1.MsgWrappedBeginRedelegate".to_string(),
        value: Binary::from(restaking_msg.encode_to_vec()),
    })
}

#[allow(clippy::type_complexity)]
pub fn get_staker_undelegation(
    storage: &mut dyn Storage,
    total_received_amount: Uint128,
    unbonding_records: &mut [UnbondRecord],
    total_liquid_stake: Uint128,
    block_height: u64,
) -> Result<
    (
        HashMap<(String, String), StakerUndelegation>,
        Vec<u64>,
        Uint128,
    ),
    ContractError,
> {
    let total_received_amount_in_decimal =
        Decimal::from_ratio(total_received_amount, Uint128::one());
    let mut unbond_record_ids = vec![];

    // hash map with tuple of staker and recipient as key
    let mut staker_undelegation: HashMap<(String, String), StakerUndelegation> = HashMap::new();

    for record in unbonding_records.iter_mut() {
        let record_recipient_ibc_channel_id = UNBOND_RECIPIENT_IBC_CHANNEL
            .load(storage, record.id)
            .unwrap_or(None);

        let entry = staker_undelegation
            .entry((
                record.staker.clone(),
                record.recipient.clone().unwrap_or("".to_string()),
            ))
            .and_modify(|e| e.unstake_amount += record.amount)
            .or_insert(StakerUndelegation {
                unstake_amount: record.amount,
                channel_id: record.channel_id,
                unstake_return_native_amount: None,
                recipient: record.recipient.clone(),
                recipient_channel_id: record.recipient_channel_id,
                recipient_ibc_channel_id: record_recipient_ibc_channel_id,
            });

        let user_to_total_unstake_ratio =
            Decimal::from_ratio(entry.unstake_amount, total_liquid_stake);

        let unstake_return_native_amount =
            (user_to_total_unstake_ratio * total_received_amount_in_decimal).to_uint_floor();

        entry.unstake_return_native_amount = Some(unstake_return_native_amount);

        record.released = true;

        record.released_height = block_height;

        unbond_record().save(storage, record.id, record)?;

        unbond_record_ids.push(record.id);
    }

    // released amount before adjusted with dust distribution, sometime it can be lower than total received amount
    let released_amount: Uint128 = staker_undelegation
        .values()
        .map(|item| item.unstake_return_native_amount.unwrap())
        .sum();

    let dust_amount = (total_received_amount_in_decimal
        - Decimal::from_ratio(released_amount, Uint128::one()))
    .to_uint_floor();

    let mut total_released_amount = Uint128::zero();
    let dust_distribution = calc::calculate_dust_distribution(
        dust_amount,
        Uint128::new(unbonding_records.len() as u128),
    );
    for (i, record) in unbonding_records.iter_mut().enumerate() {
        let recipient = match record.recipient.clone() {
            Some(recipient) => recipient,
            None => "".to_string(),
        };
        let staker_undelegation =
            match staker_undelegation.get_mut(&(record.staker.clone(), recipient)) {
                Some(x) => x,
                None => continue,
            };
        let dust = dust_distribution[i];
        staker_undelegation.unstake_return_native_amount = staker_undelegation
            .unstake_return_native_amount
            .map(|x| x + dust);

        total_released_amount += staker_undelegation.unstake_return_native_amount.unwrap();
    }

    Ok((
        staker_undelegation,
        unbond_record_ids,
        total_released_amount,
    ))
}

/// Restake to delegate base on amount and update exchange rate
pub fn inject(
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
    delegator: Addr,
    amount: Uint128,
    params: Parameters,
    block_height: u64,
) -> Result<(Vec<CosmosMsg>, InjectData), ContractError> {
    if amount < params.min_bond {
        return Err(ContractError::BondAmountTooLow {});
    }

    let validators_reg = crate::state::VALIDATORS_REGISTRY.load(storage)?;

    let coin_denom = params.underlying_coin_denom.to_string();
    let msgs = delegation::get_delegate_to_validator_msgs(
        delegator.to_string(),
        amount,
        coin_denom.to_string(),
        validators_reg.validators.clone(),
    );

    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    // logic to mint token and update the supply and total_bond_amount
    let mut state = STATE.load(storage)?;

    let delegated_amount = get_actual_total_delegated(
        querier,
        delegator.to_string(),
        &coin_denom,
        &validators_list,
    )?;

    state.total_delegated_amount = delegated_amount;
    let unclaimed_reward = get_unclaimed_reward(
        querier,
        delegator.to_string(),
        &coin_denom,
        &validators_list,
    )?;

    let reward_balance =
        calc::normalize_reward_balance(storage, block_height, unclaimed_reward).unwrap();

    let reward = reward_balance + unclaimed_reward;
    let fee = calculate_fee_from_reward(reward, params.fee_rate);
    let total_bond_amount = delegated_amount + reward - fee;

    let mut supply_queue: SupplyQueue = SUPPLY_QUEUE.load(storage)?;
    calc::normalize_supply_queue(&mut supply_queue, block_height);
    let exchange_rate = if total_bond_amount == Uint128::zero() {
        Decimal::one()
    } else {
        calc::calculate_exchange_rate(total_bond_amount, state.total_supply, &supply_queue)
    };

    if exchange_rate < Decimal::one() {
        return Err(ContractError::InvalidExchangeRate {});
    }

    let prev_exchange_rate = exchange_rate;
    let new_bond_amount = total_bond_amount + amount;
    let new_exchange_rate = if total_bond_amount == Uint128::zero() {
        Decimal::one()
    } else {
        calc::calculate_exchange_rate(new_bond_amount, state.total_supply, &supply_queue)
    };

    state.total_bond_amount = new_bond_amount;
    state.total_delegated_amount += amount;
    state.exchange_rate = exchange_rate;
    STATE.save(storage, &state)?;

    let data = InjectData {
        prev_exchange_rate,
        new_exchange_rate,
        total_supply: state.total_supply,
        reward_balance,
        unclaimed_reward,
        delegated_amount: state.total_delegated_amount,
        total_bond_amount: state.total_bond_amount,
    };

    Ok((msgs, data))
}
