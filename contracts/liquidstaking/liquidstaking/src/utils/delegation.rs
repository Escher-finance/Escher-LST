use crate::event::SubmitBatchEvent;
use crate::event::UnbondEventsFromAtts;
use crate::event::UnstakeRequestEvent;
use crate::msg::ValidatorDelegation;
use crate::utils::calc;
use crate::utils::delegation;
use crate::utils::token;
use crate::ContractError;
use crate::{
    msg::{BondData, DelegationDiff, MintTokensPayload},
    state::{
        increment_tokens, unbond_record, Parameters, UnbondRecord, Validator, ValidatorsRegistry,
        PARAMETERS, PENDING_BATCH_ID, QUOTE_TOKEN, REWARD_BALANCE, STATE, VALIDATORS_REGISTRY,
    },
};
use cosmwasm_std::Attribute;
use cosmwasm_std::BankMsg;
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

use super::batch::{batches, Batch, BatchStatus};
use super::protocol;

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

/// Convert Uint256 to Uint128
pub fn to_uint128(v: Uint256) -> StdResult<Uint128> {
    Uint128::from_str(&v.to_string())
}

// for testing only
pub fn get_mock_total_reward(total_bond_amount: Uint128) -> Uint128 {
    let ratio = Decimal::from_ratio(Uint128::new(1000), Uint128::new(1005));
    calc::calculate_staking_token_from_rate(total_bond_amount, ratio)
}

/// return how much to undelegate native token from ratio of total delegated amount divide with total bond with reward value amount
/// NOTE: Not used
pub fn calculate_undelegate_amount(
    native_token_amount: Uint128,
    delegated_amount: Uint128,
    total_bonded_amount: Uint128,
) -> Uint128 {
    let native_token_undelegate_decimal = Decimal::from_ratio(native_token_amount, Uint128::one());
    let ratio = Decimal::from_ratio(delegated_amount, total_bonded_amount);

    let undelegate_native_decimal = native_token_undelegate_decimal * ratio;
    undelegate_native_decimal.to_uint_floor()
}

/// NOTE: This is the same calculation as `utils:calculate_native_token_from_staking_token`
pub fn calculate_delegated_amount(amount: Uint128, ratio: Decimal) -> Uint128 {
    (ratio * Decimal::from_ratio(amount, Uint128::one())).to_uint_floor()
}

pub fn get_undelegate_from_validator_msgs(
    undelegate_amount: Uint128,
    coin_denom: String,
    validators: Vec<Validator>,
) -> (Uint128, Vec<CosmosMsg>, Vec<Attribute>) {
    let mut msgs: Vec<CosmosMsg> = vec![];

    let total_weight = Uint128::from(
        validators
            .iter()
            .map(|v| v.weight)
            .reduce(|a, b| (a + b))
            .unwrap_or(0),
    );

    let mut atts = vec![];

    let mut total_undelegate_amount = Uint128::zero();
    for validator in validators.into_iter() {
        let ratio = Decimal::from_ratio(Uint128::from(validator.weight), total_weight);

        let undelegate_amount_dec = Decimal::from_ratio(undelegate_amount, Uint128::one());
        let undelegate_amount_for_validator = (undelegate_amount_dec * ratio).to_uint_floor();

        let amount = Coin {
            amount: undelegate_amount_for_validator.clone(),
            denom: coin_denom.to_string(),
        };
        let undelegate_staking_msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: validator.address.to_string(),
            amount,
        });
        msgs.push(undelegate_staking_msg);
        atts.push(Attribute {
            key: validator.address.to_string(),
            value: undelegate_amount_for_validator.to_string(),
        });

        total_undelegate_amount += undelegate_amount_for_validator;
    }

    (total_undelegate_amount, msgs, atts)
}

pub fn get_validator_delegation_map_with_total_bond(
    deps: DepsMut,
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

        let delegation_amount = calculate_delegated_amount(total_delegated_amount, ratio);
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

        let delegate_amount = calculate_delegated_amount(delegate_amount, ratio);

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
        get_validator_delegation_map_with_total_bond(deps, delegator.to_string(), prev_validators)?;

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
    let coin_denom = params.underlying_coin_denom.to_string();
    let msgs = delegation::get_delegate_to_validator_msgs(
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
        let unclaimed_reward = get_unclaimed_reward(
            querier,
            delegator.to_string(),
            coin_denom.clone(),
            validators_list,
        )?;

        let contract_reward_balance = REWARD_BALANCE.load(storage)?;
        let reward = unclaimed_reward + contract_reward_balance;

        total_bond_amount = delegated_amount + reward;
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

/// Process unbond that will burn liquid staking token, undelegate some amount from validator according to exchange rate and create UnbondRecord
// pub fn process_unbond(
//     env: Env,
//     storage: &mut dyn Storage,
//     querier: QuerierWrapper,
//     sender: String,
//     staker: String,
//     delegator: Addr,
//     unbond_amount: Uint128,
//     params: Parameters,
//     validators_reg: ValidatorsRegistry,
//     channel_id: Option<u32>,
// ) -> Result<(Vec<CosmosMsg>, UnbondData), ContractError> {
//     let coin_denom = params.underlying_coin_denom;

//     let validators_list: Vec<String> = validators_reg
//         .validators
//         .iter()
//         .map(|v| v.address.clone())
//         .collect();

//     let mut state = STATE.load(storage)?;

//     let delegated_amount = get_actual_total_delegated(
//         querier,
//         delegator.to_string(),
//         coin_denom.clone(),
//         validators_list.clone(),
//     );
//     state.total_delegated_amount = delegated_amount;
//     let reward = get_actual_total_reward(
//         querier,
//         delegator.to_string(),
//         coin_denom.clone(),
//         validators_list,
//         params.reward_address.into(),
//     )?;

//     let total_bond_amount = delegated_amount + reward;

//     if total_bond_amount.is_zero() || state.total_supply.is_zero() {
//         return Err(ContractError::ZeroSupplyOrDelegatedAmount {});
//     }
//     let current_exchange_rate = Decimal::from_ratio(total_bond_amount, state.total_supply);

//     // calculate how much native token undelegated amount from staked token amount base on current exchange rate
//     let undelegate_amount: Uint128 = calc::calculate_native_token_from_staking_token(
//         unbond_amount.clone(),
//         current_exchange_rate,
//     );

//     let mut msgs: Vec<CosmosMsg> = vec![];
//     if delegated_amount < undelegate_amount {
//         return Err(ContractError::NotEnoughAvailableFund {}); // this error only happen on development or sole staker and if process rewards not happen yet
//     }
//     let undelegate_msgs = get_undelegate_from_validator_msgs(
//         undelegate_amount,
//         coin_denom.clone(),
//         validators_reg.validators,
//     );
//     msgs.extend(undelegate_msgs.clone());

//     let burn_msg = token::burn_token(unbond_amount, params.cw20_address.to_string());
//     msgs.push(burn_msg.into());

//     let id: u64 = increment_tokens(storage).unwrap();
//     let history = UnbondRecord {
//         id,
//         height: env.block.height,
//         channel_id,
//         sender: sender.clone(),
//         staker: staker.clone(),
//         amount: unbond_amount,
//         undelegate_amount: undelegate_amount,
//         created: env.block.time,
//         released_height: 0,
//         released: false,
//     };
//     unbond_record().save(storage, id, &history)?;

//     // // update total bond, supply and exchange rate here
//     state.total_bond_amount = total_bond_amount - undelegate_amount;
//     state.total_supply = state.total_supply - unbond_amount;
//     state.total_delegated_amount = delegated_amount - undelegate_amount;
//     state.update_exchange_rate();
//     STATE.save(storage, &state)?;

//     Ok((
//         msgs,
//         UnbondData {
//             record_id: id,
//             undelegate_amount: undelegate_amount,
//             delegated_amount: state.total_delegated_amount,
//             reward: reward,
//             exchange_rate: current_exchange_rate,
//             total_supply: state.total_supply,
//         },
//     ))
// }

/// Process unstake requests from batch that will burn liquid staking token, undelegate some amount from validator according to exchange rate and create UnbondRecord
/// 1. Delegate to validators
/// 2. Set current batch status to submitted
/// 3. Create new SubmitBatchEvent
/// 4. Create new pending batch
pub fn submit_pending_batch(
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
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

    let mut state = STATE.load(storage)?;

    let delegated_amount = get_actual_total_delegated(
        querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list.clone(),
    )?;
    state.total_delegated_amount = delegated_amount;
    // query the total reward from this contract
    let unclaimed_reward = get_unclaimed_reward(
        querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list,
    )?;

    let contract_reward_balance = REWARD_BALANCE.load(storage)?;
    let reward = unclaimed_reward + contract_reward_balance;

    let total_bond_amount = delegated_amount + reward;

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
    let (total_undelegate_amount, undelegate_msgs, atts) = get_undelegate_from_validator_msgs(
        undelegate_amount,
        coin_denom.clone(),
        validators_reg.validators,
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
    STATE.save(storage, &state)?;

    let next_action_time = time.seconds() + params.unbonding_time;
    batch.expected_native_unstaked = Some(total_undelegate_amount);
    batch.update_status(BatchStatus::Submitted, Some(next_action_time));

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
    );

    events.push(ev);

    // Create new pending batch
    let new_pending_batch = Batch::new(
        batch.id + 1,
        Uint128::zero(),
        time.seconds() + params.batch_period,
    );
    batches().save(storage, new_pending_batch.id, &new_pending_batch)?;
    PENDING_BATCH_ID.save(storage, &new_pending_batch.id)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::assert_approx_eq;

    #[test]
    fn test_calculate_delegated_amount() {
        let amount = Uint128::new(112382);
        assert_eq!(
            calculate_delegated_amount(amount, Decimal::from_ratio(1_u128, 2_u128)),
            amount / Uint128::new(2)
        );
    }

    #[test]
    fn test_get_undelegate_from_validator_msgs() {
        let undelegate_amount = Uint128::from(500642u32);
        let coin_denom = "muno".to_string();

        let validators = vec![
            Validator {
                address: "abc".to_string(),
                weight: 10,
            },
            Validator {
                address: "bcd".to_string(),
                weight: 20,
            },
        ];
        let (total_undelegate_amount, undelegate_msgs, _) =
            get_undelegate_from_validator_msgs(undelegate_amount, coin_denom.clone(), validators);
        let undelegate_msgs_unwrapped = undelegate_msgs
            .iter()
            .filter_map(|msg| {
                if let CosmosMsg::Staking(StakingMsg::Undelegate { validator, amount }) = msg {
                    return Some((validator, amount.amount));
                }
                return None;
            })
            .collect::<Vec<_>>();
        assert_eq!(undelegate_msgs_unwrapped.len(), 2);
        assert_eq!(
            undelegate_msgs_unwrapped[0].1 + undelegate_msgs_unwrapped[1].1,
            total_undelegate_amount
        );
        assert_approx_eq!(undelegate_amount, total_undelegate_amount, "0.001");
        assert_approx_eq!(
            undelegate_msgs_unwrapped[0].1 * Uint128::from(2_u128),
            undelegate_msgs_unwrapped[1].1,
            "0.001"
        );
    }

    #[test]
    fn test_get_validator_delegation_map_base_on_weight() {
        let validators = vec![
            Validator {
                address: "abc".to_string(),
                weight: 10,
            },
            Validator {
                address: "bcd".to_string(),
                weight: 20,
            },
        ];
        let total_delegated_amount = Uint128::new(250000);
        let map =
            get_validator_delegation_map_base_on_weight(validators.clone(), total_delegated_amount);
        assert_eq!(map.len(), 2);
        assert_approx_eq!(
            map.values().sum::<Uint128>(),
            total_delegated_amount,
            "0.0001"
        );
        assert_approx_eq!(
            map.get("abc").unwrap() * Uint128::new(2),
            *map.get("bcd").unwrap(),
            "0.0001"
        );

        assert_eq!(
            HashMap::new(),
            get_validator_delegation_map_base_on_weight(vec![], total_delegated_amount)
        );
        assert_eq!(
            HashMap::from([
                (validators[0].address.clone(), Uint128::zero()),
                (validators[1].address.clone(), Uint128::zero())
            ]),
            get_validator_delegation_map_base_on_weight(validators, Uint128::zero())
        );
    }

    #[test]
    fn test_get_surplus_deficit_validators() {
        let validator_a = "a".to_string();
        let validator_b = "b".to_string();
        let validator_c = "c".to_string();
        let validator_d = "d".to_string();
        let amount_a = Uint128::new(100);
        let amount_b = Uint128::new(1000);
        let amount_c = Uint128::new(250);
        let correct_amount_b = Uint128::new(800);
        let correct_amount_c = Uint128::new(350);
        let correct_amount_d = Uint128::new(500);
        let validator_delegation_map = HashMap::from([
            (validator_a.clone(), amount_a),
            (validator_b.clone(), amount_b),
            (validator_c.clone(), amount_c),
        ]);
        let correct_validator_delegation_map = HashMap::from([
            (validator_b.clone(), correct_amount_b),
            (validator_c.clone(), correct_amount_c),
            (validator_d.clone(), correct_amount_d),
        ]);
        let (mut surplus_validators, mut deficit_validators) = get_surplus_deficit_validators(
            validator_delegation_map,
            correct_validator_delegation_map,
        );
        surplus_validators.sort_by_key(|v| v.address.clone());
        deficit_validators.sort_by_key(|v| v.address.clone());
        assert_eq!(
            surplus_validators,
            vec![
                ValidatorDelegation {
                    address: validator_a.clone(),
                    delegation_diff: DelegationDiff::Surplus,
                    diff_amount: amount_a,
                },
                ValidatorDelegation {
                    address: validator_b.clone(),
                    delegation_diff: DelegationDiff::Surplus,
                    diff_amount: amount_b.abs_diff(correct_amount_b),
                },
            ]
        );
        assert_eq!(
            deficit_validators,
            vec![
                ValidatorDelegation {
                    address: validator_c.clone(),
                    delegation_diff: DelegationDiff::Deficit,
                    diff_amount: amount_c.abs_diff(correct_amount_c),
                },
                ValidatorDelegation {
                    address: validator_d.clone(),
                    delegation_diff: DelegationDiff::Deficit,
                    diff_amount: correct_amount_d,
                },
            ]
        );
    }

    #[test]
    fn test_get_restaking_msgs() {
        let surplus_validators = Vec::from([
            ValidatorDelegation {
                address: "a".to_string(),
                delegation_diff: DelegationDiff::Surplus,
                diff_amount: Uint128::new(200),
            },
            ValidatorDelegation {
                address: "b".to_string(),
                delegation_diff: DelegationDiff::Surplus,
                diff_amount: Uint128::new(900),
            },
        ]);
        let deficient_validators = vec![
            ValidatorDelegation {
                address: "c".to_string(),
                delegation_diff: DelegationDiff::Deficit,
                diff_amount: Uint128::new(1000),
            },
            ValidatorDelegation {
                address: "d".to_string(),
                delegation_diff: DelegationDiff::Deficit,
                diff_amount: Uint128::new(500),
            },
        ];
        let msgs = get_restaking_msgs(
            surplus_validators,
            deficient_validators,
            "denom".to_string(),
        );
        assert!(msgs.len() > 0);
        let zero_redelegate = msgs.iter().find(|msg| {
            if let CosmosMsg::Staking(StakingMsg::Redelegate {
                src_validator: _,
                dst_validator: _,
                amount,
            }) = msg
            {
                return amount.amount.is_zero();
            }
            false
        });
        // Amount cannot be zero
        assert!(zero_redelegate.is_none());

        let surplus_validators = Vec::from([
            ValidatorDelegation {
                address: "a".to_string(),
                delegation_diff: DelegationDiff::Surplus,
                diff_amount: Uint128::new(5000),
            },
            ValidatorDelegation {
                address: "b".to_string(),
                delegation_diff: DelegationDiff::Surplus,
                diff_amount: Uint128::new(5000),
            },
        ]);
        let deficient_validators = vec![
            ValidatorDelegation {
                address: "c".to_string(),
                delegation_diff: DelegationDiff::Deficit,
                diff_amount: Uint128::new(7500),
            },
            ValidatorDelegation {
                address: "d".to_string(),
                delegation_diff: DelegationDiff::Deficit,
                diff_amount: Uint128::new(2500),
            },
        ];
        let msgs = get_restaking_msgs(
            surplus_validators,
            deficient_validators,
            "denom".to_string(),
        );
        let mut net_amounts = msgs
            .into_iter()
            .filter_map(|msg| {
                if let CosmosMsg::Staking(StakingMsg::Redelegate {
                    src_validator: _,
                    dst_validator,
                    amount,
                }) = msg
                {
                    return Some((dst_validator, amount.amount.u128()));
                }
                None
            })
            .fold(HashMap::new(), |mut h, pair| {
                h.entry(pair.0)
                    .and_modify(|amount| *amount += pair.1)
                    .or_insert(pair.1);
                h
            })
            .into_iter()
            .collect::<Vec<_>>();
        net_amounts.sort_by_key(|a| a.1);
        // Should redelegate in totality
        assert_eq!(
            net_amounts,
            vec![("d".to_string(), 2500_u128), ("c".to_string(), 7500_u128)]
        );
    }

    #[test]
    fn test_get_delegate_to_validator_msgs() {
        let delegate_amount = Uint128::new(50000);
        let coin_denom = "denom".to_string();
        let validators = vec![
            Validator {
                address: "abc".to_string(),
                weight: 10,
            },
            Validator {
                address: "bcd".to_string(),
                weight: 20,
            },
        ];
        let msgs = get_delegate_to_validator_msgs(delegate_amount, coin_denom, validators);
        let amounts = msgs
            .into_iter()
            .filter_map(|msg| {
                if let CosmosMsg::Staking(StakingMsg::Delegate { validator, amount }) = msg {
                    return Some((validator, amount.amount));
                }
                None
            })
            .fold(HashMap::new(), |mut h, pair| {
                h.entry(pair.0)
                    .and_modify(|amount| *amount += pair.1)
                    .or_insert(pair.1);
                h
            });
        assert_approx_eq!(
            amounts.get("abc").unwrap() * Uint128::new(2),
            *amounts.get("bcd").unwrap(),
            "0.001"
        );
        assert_eq!(
            amounts.iter().map(|a| a.1).sum::<Uint128>(),
            delegate_amount
        );
    }

    #[test]
    fn test_get_delegate_to_validator_msgs_should_skip_zero_delegate_amount() {
        let validators = Vec::from([
            Validator {
                address: "a".to_string(),
                weight: 0,
            },
            Validator {
                address: "b".to_string(),
                weight: 9,
            },
            Validator {
                address: "c".to_string(),
                weight: 1,
            },
        ]);
        let msgs = get_delegate_to_validator_msgs(
            Uint128::from(100_u128),
            "denom".to_string(),
            validators,
        );

        let zero_amount_msg = msgs.iter().find(|msg| {
            if let CosmosMsg::Staking(StakingMsg::Delegate {
                validator: _,
                amount,
            }) = msg
            {
                if amount.amount == Uint128::zero() {
                    return true;
                }
            }
            false
        });
        assert!(zero_amount_msg.is_none());
    }

    #[test]
    fn test_get_validator_delegation_map_base_on_weight_should_delegate_remaining_amount() {
        let validators = Vec::from([
            Validator {
                address: "a".to_string(),
                weight: 1,
            },
            Validator {
                address: "b".to_string(),
                weight: 100,
            },
            Validator {
                address: "c".to_string(),
                weight: 1000,
            },
        ]);
        let total_delegated_amount = Uint128::from(500000_u128);

        assert_eq!(
            get_validator_delegation_map_base_on_weight(validators, total_delegated_amount)
                .iter()
                .map(|(_addr, amount)| amount)
                .sum::<Uint128>(),
            total_delegated_amount
        )
    }
}
