use crate::event::SubmitBatchEvent;
use crate::event::UnbondEventsFromAtts;
use crate::event::UnstakeRequestEvent;
use crate::msg::ValidatorDelegation;
use crate::utils::batch::{batches, Batch, BatchStatus};
use crate::utils::calc;
use crate::utils::calc::to_uint128;
use crate::utils::protocol;
use crate::utils::token;
use crate::ContractError;
use crate::{
    msg::{BondData, DelegationDiff, MintTokensPayload},
    state::{
        increment_tokens, unbond_record, Parameters, UnbondRecord, Validator, ValidatorsRegistry,
        EXECUTOR, PARAMETERS, PENDING_BATCH_ID, QUOTE_TOKEN, REWARD_BALANCE, STATE,
        VALIDATORS_REGISTRY,
    },
};
use cosmwasm_std::Attribute;
use cosmwasm_std::BankMsg;
use cosmwasm_std::Deps;
use cosmwasm_std::Event;
use cosmwasm_std::Timestamp;
use cosmwasm_std::{
    to_json_binary, Addr, Coin, CosmosMsg, Decimal, DepsMut, Env, QuerierWrapper, StakingMsg,
    StdResult, Storage, SubMsg, Uint128, Uint256,
};
use unionlabs_primitives::Bytes;
use unionlabs_primitives::H256;

use std::collections::HashMap;
use std::str::FromStr;

use super::calc::calculate_native_token_from_staking_token;

pub const DEFAULT_TIMEOUT_TIMESTAMP_OFFSET: u64 = 600;

/// get total delegated token value from validators in native token
pub fn get_actual_total_delegated(
    querier: QuerierWrapper,
    delegator: String,
    denom: String,
    validators: Vec<String>,
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

pub fn get_unclaimed_reward(
    querier: QuerierWrapper,
    delegator: String,
    denom: String,
    validators: Vec<String>,
) -> StdResult<Uint128> {
    let mut total_rewards = Uint128::new(0);
    let result = querier.query_delegation_total_rewards(delegator)?;

    for delegator_reward in result.rewards {
        if validators.contains(&delegator_reward.validator_address) {
            for reward in delegator_reward.reward {
                if reward.denom == denom {
                    let reward_val = to_uint128(reward.amount.to_uint_floor())?;
                    total_rewards += reward_val;
                }
            }
        }
    }

    // add query reward contract balance
    Ok(total_rewards)
}

// for testing only
pub fn get_mock_total_reward(total_bond_amount: Uint128) -> Uint128 {
    let ratio = Decimal::from_ratio(Uint128::new(1000), Uint128::new(1005));
    calc::calculate_staking_token_from_rate(total_bond_amount, ratio)
}

pub fn get_undelegate_msgs(
    undelegate_amount: Uint128,
    coin_denom: String,
    validator_delegation_ratio: HashMap<String, Decimal>,
) -> (Uint128, Vec<CosmosMsg>, Vec<Attribute>) {
    let mut msgs: Vec<CosmosMsg> = vec![];

    let mut atts = vec![];

    let mut total_undelegate_amount = Uint128::zero();

    let undelegate_amount_dec: Decimal = Decimal::from_ratio(undelegate_amount, Uint128::one());

    for (validator, ratio) in validator_delegation_ratio.into_iter() {
        let undelegate_amount_for_validator = (undelegate_amount_dec * ratio).to_uint_floor();
        if undelegate_amount_for_validator.is_zero() {
            continue;
        }
        let amount = Coin {
            amount: undelegate_amount_for_validator.clone(),
            denom: coin_denom.to_string(),
        };
        let undelegate_staking_msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: validator.clone(),
            amount,
        });
        msgs.push(undelegate_staking_msg);

        atts.push(Attribute {
            key: validator,
            value: undelegate_amount_for_validator.to_string(),
        });

        total_undelegate_amount += undelegate_amount_for_validator;
    }

    (total_undelegate_amount, msgs, atts)
}

pub fn get_validator_delegation_map_with_total_bond(
    deps: Deps,
    delegator: String,
    validators: Vec<Validator>,
) -> Result<(HashMap<String, Uint128>, Uint128), ContractError> {
    let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    let mut total_delegated_amount = Uint128::zero();

    for validator in validators {
        let validator_bond = deps
            .querier
            .query_delegation(delegator.clone(), validator.address.clone())?;

        let delegation_amount = match validator_bond {
            Some(delegation) => delegation.amount.amount,
            None => Uint128::zero(),
        };
        validator_delegation_map.insert(validator.address.to_string(), delegation_amount);
        total_delegated_amount += delegation_amount;
    }

    Ok((validator_delegation_map, total_delegated_amount))
}

pub fn get_validator_delegation_map_base_on_weight(
    validators: Vec<Validator>,
    total_delegated_amount: Uint128,
) -> HashMap<String, Uint128> {
    let total_weight = validators
        .iter()
        .map(|v| v.weight)
        .reduce(|a, b| (a + b))
        .unwrap_or(0);

    let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    let mut total_delegation_amount = Uint128::zero();
    let mut first_validator = String::new();
    for validator in validators {
        let ratio = Decimal::from_ratio(validator.weight, total_weight);

        let delegation_amount =
            calculate_native_token_from_staking_token(total_delegated_amount, ratio);
        total_delegation_amount += delegation_amount;

        let validator_address = validator.address.to_string();
        if first_validator.is_empty() {
            first_validator = validator_address.clone();
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

pub fn get_surplus_deficit_validators(
    validator_delegation_map: HashMap<String, Uint128>,
    correct_validator_delegation_map: HashMap<String, Uint128>,
) -> (Vec<ValidatorDelegation>, Vec<ValidatorDelegation>) {
    let mut surplus_validators: Vec<ValidatorDelegation> = vec![];
    let mut deficient_validators: Vec<ValidatorDelegation> = vec![];
    for (key, previous_amount) in validator_delegation_map.clone().iter_mut() {
        // check if old validator key exists on new validators map
        if correct_validator_delegation_map.get(key).is_none() {
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
                diff_amount: correct_amount.clone(),
            }),
        }
    }

    (surplus_validators, deficient_validators)
}

pub fn get_restaking_msgs(
    mut surplus_validators: Vec<ValidatorDelegation>,
    mut deficient_validators: Vec<ValidatorDelegation>,
    denom: String,
) -> Vec<CosmosMsg> {
    let mut msgs: Vec<CosmosMsg> = vec![];

    surplus_validators.sort_by(|a, b| b.diff_amount.cmp(&a.diff_amount));
    deficient_validators.sort_by_key(|a| a.diff_amount);

    for surplus_validator in surplus_validators.iter_mut() {
        for deficient_validator in deficient_validators.iter_mut() {
            if surplus_validator.diff_amount < deficient_validator.diff_amount {
                if surplus_validator.diff_amount.is_zero() {
                    continue;
                }

                // the deficit amount higher than surplus amount so we can restake all surplus amount
                let redelegate_msg = CosmosMsg::Staking(StakingMsg::Redelegate {
                    src_validator: surplus_validator.address.clone(),
                    dst_validator: deficient_validator.address.clone(),
                    amount: Coin {
                        amount: surplus_validator.diff_amount,
                        denom: denom.clone(),
                    },
                });
                surplus_validator.diff_amount = Uint128::from(0u32);
                deficient_validator.diff_amount =
                    deficient_validator.diff_amount - surplus_validator.diff_amount;

                msgs.push(redelegate_msg);
            } else {
                if deficient_validator.diff_amount.is_zero() {
                    continue;
                }

                println!("{:?} <> {:?}", surplus_validator, deficient_validator);

                let redelegate_msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Redelegate {
                    src_validator: surplus_validator.address.clone(),
                    dst_validator: deficient_validator.address.clone(),
                    amount: Coin {
                        amount: deficient_validator.diff_amount,
                        denom: denom.clone(),
                    },
                });

                surplus_validator.diff_amount =
                    surplus_validator.diff_amount - deficient_validator.diff_amount;
                deficient_validator.diff_amount = Uint128::from(0u32);
                msgs.push(redelegate_msg);
            }
        }
    }

    msgs
}

pub fn get_delegate_to_validator_msgs(
    delegate_amount: Uint128,
    coin_denom: String,
    validators: Vec<Validator>,
) -> Vec<CosmosMsg> {
    let total_weight = validators
        .iter()
        .map(|v| v.weight)
        .reduce(|a, b| (a + b))
        .unwrap_or(1);

    let mut total_delegated: Uint128 = Uint128::from(0u32);

    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut first_validator = String::new();

    for validator in validators {
        let ratio =
            Decimal::from_ratio(Uint128::from(validator.weight), Uint128::from(total_weight));

        let delegate_amount = calculate_native_token_from_staking_token(delegate_amount, ratio);

        if delegate_amount == Uint128::zero() {
            continue;
        }

        total_delegated += delegate_amount;
        let amount = Coin {
            amount: delegate_amount.clone(),
            denom: coin_denom.to_string(),
        };
        let staking_msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Delegate {
            validator: validator.address.to_string(),
            amount,
        });

        msgs.push(staking_msg.into());

        if first_validator.is_empty() {
            first_validator = validator.address.to_string();
        }
    }

    // calculate remaining

    let remaining_amount = delegate_amount - total_delegated;
    if !remaining_amount.is_zero() {
        let remaining_staking_msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Delegate {
            validator: first_validator,
            amount: Coin {
                denom: coin_denom.clone(),
                amount: remaining_amount,
            },
        });

        msgs.push(remaining_staking_msg.into());
    }
    msgs
}

pub fn get_unbond_all_messages(
    deps: DepsMut,
    delegator: Addr,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let delegations_resp = deps.querier.query_all_delegations(delegator);
    let params = PARAMETERS.load(deps.storage)?;
    let denom = params.underlying_coin_denom;

    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let mut msgs: Vec<CosmosMsg> = vec![];
    for (_pos, validator) in validators_reg.validators.iter().enumerate() {
        let undelegate_amount: Uint128 = delegations_resp
            .as_ref()
            .unwrap()
            .into_iter()
            .filter(|d| {
                d.amount.denom == denom
                    && !d.amount.amount.is_zero()
                    && d.validator == validator.address
            })
            .map(|d| d.amount.amount)
            .sum();

        let amount = Coin {
            amount: undelegate_amount.clone(),
            denom: denom.to_string(),
        };
        let undelegate_staking_msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: validator.address.to_string(),
            amount,
        });

        msgs.push(undelegate_staking_msg.into());
    }

    Ok(msgs)
}

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
            delegator.to_string(),
            prev_validators,
        )?;

    let correct_validator_delegation_map =
        get_validator_delegation_map_base_on_weight(validators, total_delegated_amount);

    let (surplus_validators, deficient_validators) =
        get_surplus_deficit_validators(validator_delegation_map, correct_validator_delegation_map);

    let msgs: Vec<CosmosMsg> = get_restaking_msgs(surplus_validators, deficient_validators, denom);

    Ok(msgs)
}

/// Process bond call to mint liquid staking token, delegate/stake base on the bond amount and exchange rate
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
) -> Result<(Vec<CosmosMsg>, Vec<SubMsg>, BondData), ContractError> {
    if amount < params.min_bond {
        return Err(ContractError::BondAmountTooLow {});
    }

    let coin_denom = params.underlying_coin_denom.to_string();
    let msgs = get_delegate_to_validator_msgs(
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
        coin_denom.clone(),
        validators_list.clone(),
    )?;

    let total_bond_amount: Uint128;

    if !cfg!(test) {
        state.total_delegated_amount = delegated_amount;
        // query the total reward from this contract
        let reward = get_actual_total_reward(
            storage,
            querier,
            delegator.to_string(),
            coin_denom.clone(),
            validators_list,
        )?;
        let fee = calc::calc_with_rate(reward, params.fee_rate);
        total_bond_amount = delegated_amount + reward - fee;

        // update the reward balance on this contract as there is automatic reward withdrawal on delegation
        REWARD_BALANCE.save(storage, &reward)?;
    } else {
        total_bond_amount = get_mock_total_reward(state.total_bond_amount);
    }

    let mut exchange_rate = state.exchange_rate;

    if !total_bond_amount.is_zero() && !state.total_supply.is_zero() {
        exchange_rate = Decimal::from_ratio(total_bond_amount, state.total_supply);
    }

    let mint_amount = calc::calculate_staking_token_from_rate(amount, exchange_rate);

    // after update exchange rate we update the state
    state.bond_counter = state.bond_counter + 1;
    state.total_bond_amount = total_bond_amount + amount;
    state.total_supply += mint_amount;
    state.total_delegated_amount += amount;
    state.last_bond_time = bond_time;
    state.update_exchange_rate();

    STATE.save(storage, &state)?;

    let mut sub_msgs: Vec<SubMsg> = vec![];
    let payload = MintTokensPayload {
        sender: sender.to_string(),
        staker: staker.clone(),
        amount: mint_amount,
        salt,
        channel_id,
    };
    let payload_bin = to_json_binary(&payload)?;

    if !cfg!(test) {
        // Start to mint according to staked token only if it is not test
        let sub_msg: SubMsg = token::get_staked_token_submsg(
            delegator.to_string(),
            delegator.to_string(),
            mint_amount,
            params.liquidstaking_denom.clone(),
            payload_bin,
            params.cw20_address,
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
        },
    ))
}

/// Process unstake requests from batch that will burn liquid staking token, undelegate some amount from validator according to exchange rate and create UnbondRecord
/// 1. Undelegate to validators
/// 2. Set current batch status to submitted
/// 3. Create new SubmitBatchEvent
/// 4. Create new pending batch
pub fn submit_pending_batch(
    deps: DepsMut,
    time: Timestamp,
    sender: Addr,
    delegator: Addr,
    batch: &mut Batch,
    params: Parameters,
    validators_reg: ValidatorsRegistry,
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
            delegator.to_string(),
            validators_reg.validators.clone(),
        )?;

    state.total_delegated_amount = delegated_amount;
    // query the unclaimed total reward

    let reward = get_actual_total_reward(
        deps.storage,
        deps.querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list,
    )?;

    let fee = calc::calc_with_rate(reward, params.fee_rate);
    let total_bond_amount = delegated_amount + reward - fee;

    // update the reward balance on this contract as there is automatic reward withdrawal on undelegation
    REWARD_BALANCE.save(deps.storage, &reward)?;

    if total_bond_amount.is_zero() || state.total_supply.is_zero() {
        return Err(ContractError::ZeroSupplyOrDelegatedAmount {});
    }
    let current_exchange_rate = Decimal::from_ratio(total_bond_amount, state.total_supply);

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

    for (validator, amount) in validator_delegation_map.into_iter() {
        let ratio = Decimal::from_ratio(amount, delegated_amount);
        validators_delegation_ratio.insert(validator, ratio);
    }

    let (total_undelegate_amount, undelegate_msgs, atts) = get_undelegate_msgs(
        undelegate_amount,
        coin_denom.clone(),
        validators_delegation_ratio,
    );
    msgs.extend(undelegate_msgs.clone());

    let mut events = UnbondEventsFromAtts(atts, batch.id, time);

    let burn_msg = token::burn_token(batch.total_liquid_stake, params.cw20_address.to_string());
    msgs.push(burn_msg.into());

    // // update total bond, supply and exchange rate here
    state.total_bond_amount = total_bond_amount - total_undelegate_amount;
    state.total_supply = state.total_supply - batch.total_liquid_stake;
    state.total_delegated_amount = delegated_amount - total_undelegate_amount;
    state.update_exchange_rate();
    STATE.save(deps.storage, &state)?;

    batch.expected_native_unstaked = Some(total_undelegate_amount);

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
        coin_denom.clone(),
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
/// 3. Create UnstakeRequest event
pub fn unstake_request_in_batch(
    env: Env,
    storage: &mut dyn Storage,
    sender: String,
    staker: String,
    unstake_amount: Uint128,
    channel_id: Option<u32>,
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

    let id: u64 = increment_tokens(storage).unwrap();
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
    };
    unbond_record().save(storage, id, &record)?;

    let event = UnstakeRequestEvent(
        sender,
        staker,
        channel_id,
        unstake_amount,
        record.id,
        pending_batch.id,
        env.block.time,
    );

    Ok(event)
}

pub fn get_transfer_token_cosmos_msg(
    storage: &mut dyn Storage,
    staker: String,
    channel_id: Option<u32>,
    time: Timestamp,
    ucs03_relay_contract: String,
    undelegate_amount: Uint128,
    denom: String,
    salt: String,
) -> Result<CosmosMsg, ContractError> {
    // if balance exists, send to staker (it can be on same chain or other chain like evm/bera)
    let msg: CosmosMsg = {
        if channel_id.is_some() {
            let funds = vec![Coin {
                denom: denom.clone(),
                amount: undelegate_amount.clone(),
            }];

            // get quote token of native base denom (muno) on specific channel id
            let quote_token = QUOTE_TOKEN.load(storage, channel_id.unwrap())?;
            let wasm_msg = protocol::ucs03_transfer(
                time,
                ucs03_relay_contract.as_str().into(),
                channel_id.unwrap(),
                Bytes::from_str(staker.as_str()).unwrap(),
                denom.clone(),
                undelegate_amount,
                Bytes::from_str(quote_token.quote_token.as_str()).unwrap(),
                Uint256::from(undelegate_amount),
                funds,
                H256::from_str(salt.as_str()).unwrap(),
            )?;
            let msg: CosmosMsg = CosmosMsg::Wasm(wasm_msg);
            msg
        } else {
            let bank_msg = BankMsg::Send {
                to_address: staker.clone(),
                amount: vec![Coin {
                    denom: denom,
                    amount: undelegate_amount,
                }],
            };
            let msg: CosmosMsg = CosmosMsg::Bank(bank_msg);
            msg
        }
    };
    Ok(msg)
}

pub fn assert_executor(storage: &mut dyn Storage, sender: Addr) -> Result<(), ContractError> {
    let executor = EXECUTOR.load(storage)?;

    if sender != executor {
        return Err(ContractError::Unauthorized {});
    }

    Ok(())
}

pub fn get_actual_total_reward(
    storage: &dyn Storage,
    querier: QuerierWrapper,
    delegator: String,
    denom: String,
    validators: Vec<String>,
) -> StdResult<Uint128> {
    let unclaimed_reward = get_unclaimed_reward(querier, delegator, denom, validators)?;
    let reward_balance = REWARD_BALANCE.load(storage)?;
    Ok(unclaimed_reward + reward_balance)
}
