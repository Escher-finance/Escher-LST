use crate::event::BondEvent;
use crate::event::SubmitBatchEvent;
use crate::event::UnbondEventsFromAtts;
use crate::event::UnstakeRequestEvent;
use crate::execute::StakerUndelegation;
use crate::state::increment_tokens;
use crate::utils::authz::get_authz_ucs03_transfer;
use crate::utils::batch::{batches, Batch, BatchStatus};
use crate::utils::calc;
use crate::utils::calc::to_uint128;
use crate::utils::token;
use crate::ContractError;
use crate::{
    msg::{BondData, DelegationDiff, InjectData, ValidatorDelegation},
    state::{
        unbond_record, Parameters, UnbondRecord, Validator, ValidatorsRegistry, PARAMETERS,
        PENDING_BATCH_ID, QUOTE_TOKEN, REWARD_BALANCE, STATE, VALIDATORS_REGISTRY,
    },
};
use cosmwasm_std::Attribute;
use cosmwasm_std::Deps;
use cosmwasm_std::Event;
use cosmwasm_std::Timestamp;
use cosmwasm_std::{
    Addr, Coin, CosmosMsg, Decimal, DepsMut, Env, QuerierWrapper, StakingMsg, StdResult, Storage,
    SubMsg, Uint128,
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

                // Create a redelegate message
                msgs.push(CosmosMsg::Staking(StakingMsg::Redelegate {
                    src_validator: surplus_validator.address.clone(),
                    dst_validator: deficient_validator.address.clone(),
                    amount: Coin {
                        denom: denom.clone(),
                        amount: redelegate_amount,
                    },
                }));

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
    _sender: String,
    _staker: String,
    delegator: Addr,
    amount: Uint128,
    lst_amount: Uint128,
    bond_time: u64,
    params: Parameters,
    validators_reg: ValidatorsRegistry,
    _salt: String,
    _channel_id: Option<u32>,
    _block_height: u64,
    _recipient: Option<String>,
    _recipient_channel_id: Option<u32>,
    _on_chain_recipient: bool,
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
    let mut reward_balance: Uint128 = Uint128::zero();
    let mut unclaimed_reward: Uint128 = Uint128::zero();
    if !cfg!(test) {
        state.total_delegated_amount = delegated_amount;
        // query the total reward from this contract
        unclaimed_reward = get_unclaimed_reward(
            querier,
            delegator.to_string(),
            coin_denom.clone(),
            validators_list,
        )?;

        reward_balance = REWARD_BALANCE.load(storage)?;
        let reward = unclaimed_reward + reward_balance;
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

    if exchange_rate < Decimal::one() {
        return Err(ContractError::InvalidExchangeRate {});
    }

    // after update exchange rate we update the state
    state.bond_counter = state.bond_counter + 1;
    state.total_bond_amount = total_bond_amount + amount;
    state.total_supply += lst_amount;
    state.total_delegated_amount += amount;
    state.last_bond_time = bond_time;
    state.update_exchange_rate();

    STATE.save(storage, &state)?;

    Ok((
        msgs,
        vec![],
        BondData {
            lst_amount,
            delegated_amount: state.total_delegated_amount,
            total_bond_amount: state.total_bond_amount,
            exchange_rate,
            total_supply: state.total_supply,
            reward_balance,
            unclaimed_reward,
        },
    ))
}

/// Process staking batch
pub fn process_staking_batch(
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
    channel_id: u32,
    sender: String,
    hub_batch_id: u32,
    delegator: Addr,
    amount: Uint128,
    mint_amount: Uint128,
    bond_time: u64,
    params: Parameters,
    validators_reg: ValidatorsRegistry,
) -> Result<(Vec<CosmosMsg>, Event), ContractError> {
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
    let mut reward_balance: Uint128 = Uint128::zero();
    let mut unclaimed_reward: Uint128 = Uint128::zero();
    if !cfg!(test) {
        state.total_delegated_amount = delegated_amount;
        // query the total reward from this contract
        unclaimed_reward = get_unclaimed_reward(
            querier,
            delegator.to_string(),
            coin_denom.clone(),
            validators_list,
        )?;

        reward_balance = REWARD_BALANCE.load(storage)?;
        let reward = unclaimed_reward + reward_balance;
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

    if exchange_rate < Decimal::one() {
        return Err(ContractError::InvalidExchangeRate {});
    }

    // after update exchange rate we update the state
    state.bond_counter = state.bond_counter + 1;
    state.total_bond_amount = total_bond_amount + amount;
    state.total_supply += mint_amount;
    state.total_delegated_amount += amount;
    state.last_bond_time = bond_time;
    state.update_exchange_rate();

    STATE.save(storage, &state)?;

    let event = BondEvent(
        hub_batch_id,
        sender,
        amount,
        state.total_delegated_amount,
        mint_amount,
        state.total_bond_amount,
        state.total_supply,
        exchange_rate,
        channel_id.to_string(),
        bond_time,
        reward_balance,
        unclaimed_reward,
    );

    Ok((msgs, event))
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
    let unclaimed_reward = get_unclaimed_reward(
        deps.querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list,
    )?;

    let contract_reward_balance = REWARD_BALANCE.load(deps.storage)?;
    let reward = unclaimed_reward + contract_reward_balance;

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
    id: u64,
    sender: String,
    _staker: String,
    unstake_amount: Uint128,
    channel_id: Option<u32>,
    _recipient: Option<String>,
    _recipient_channel_id: Option<u32>,
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

    let record = UnbondRecord {
        id,
        batch_id: pending_batch.id,
        height: env.block.height,
        channel_id,
        sender: sender.clone(),
        amount: unstake_amount,
        released_height: 0,
        released: false,
        hub_batch_id: 0,
    };
    unbond_record().save(storage, id, &record)?;

    let reward_balance = REWARD_BALANCE.load(storage)?;

    let event = UnstakeRequestEvent(
        sender,
        channel_id,
        unstake_amount,
        record.id,
        pending_batch.id,
        env.block.time,
        reward_balance,
        0,
    );
    Ok(event)
}

/// Create undelegate requests that will create unbond record and put in pending batch
/// 1. Increase total liquid stake amount in pending batch
/// 2. Create unbond record and save in pending batch
/// 3. Create UndelegateRequest event
pub fn unstake_request(
    env: Env,
    storage: &mut dyn Storage,
    hub_batch_id: u32,
    sender: String,
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
        channel_id: channel_id,
        sender: sender.clone(),
        amount: unstake_amount,
        released_height: 0,
        released: false,
        hub_batch_id,
    };
    unbond_record().save(storage, id, &record)?;

    let reward_balance = REWARD_BALANCE.load(storage)?;

    let event = UnstakeRequestEvent(
        sender,
        channel_id,
        unstake_amount,
        record.id,
        pending_batch.id,
        env.block.time,
        reward_balance,
        hub_batch_id,
    );
    Ok(event)
}

pub fn get_unbonding_ucs03_transfer_cosmos_msg(
    storage: &mut dyn Storage,
    lst_contract: Addr,
    recipient: String,
    channel_id: u32,
    time: Timestamp,
    ucs03_relay_contract: String,
    undelegate_amount: Uint128,
    transfer_fee: Uint128,
    denom: String,
    salt: String,
) -> Result<CosmosMsg, ContractError> {
    let total_amount = undelegate_amount + transfer_fee;

    // for the amount
    let funds = vec![Coin {
        denom: denom.clone(),
        amount: total_amount.clone(),
    }];

    let params = PARAMETERS.load(storage)?;
    // get quote token of native base denom (muno) on specific channel id
    let quote_token = QUOTE_TOKEN.load(storage, channel_id)?;

    let quote_token_string = quote_token.quote_token.clone();

    let recipient_address = match Bytes::from_str(recipient.as_str()) {
        Ok(rec) => rec,
        Err(_) => {
            return Err(ContractError::InvalidAddress {
                kind: "recipient".into(),
                address: recipient,
            })
        }
    };
    let quote_token = match Bytes::from_str(quote_token_string.as_str()) {
        Ok(token) => token,
        Err(_) => {
            return Err(ContractError::InvalidAddress {
                kind: "quote_token".into(),
                address: quote_token_string,
            })
        }
    };

    let authz_ucs03_msg = get_authz_ucs03_transfer(
        params.cw20_address.to_string(),
        params.transfer_handler,  // granter
        lst_contract.to_string(), // grantee
        time,
        ucs03_relay_contract.as_str().into(),
        channel_id,
        recipient_address,
        denom.clone(),
        total_amount,
        quote_token,
        undelegate_amount,
        funds,
        H256::from_str(salt.as_str()).unwrap(),
        params.underlying_coin_denom.clone(),
        params.underlying_coin_denom_symbol.clone(),
        params.liquidstaking_denom.clone(),
        params.liquidstaking_denom_symbol.clone(),
    )?;

    Ok(authz_ucs03_msg)
}

pub fn get_staker_undelegation(
    storage: &mut dyn Storage,
    total_received_amount: Uint128,
    unbonding_records: &mut Vec<UnbondRecord>,
    total_liquid_stake: Uint128,
    block_height: u64,
) -> Result<(HashMap<String, StakerUndelegation>, Vec<u64>, Uint128), ContractError> {
    let total_received_amount_in_decimal =
        Decimal::from_ratio(total_received_amount, Uint128::one());
    let mut unbond_record_ids = vec![];

    // hash map with tuple of staker and recipient as key
    let mut staker_undelegation: HashMap<String, StakerUndelegation> = HashMap::new();

    for record in unbonding_records.iter_mut() {
        let entry = staker_undelegation
            .entry(record.sender.clone())
            .and_modify(|e| e.unstake_amount += record.amount)
            .or_insert(StakerUndelegation {
                unstake_amount: record.amount,
                channel_id: record.channel_id,
                unstake_return_native_amount: None,
            });

        let user_to_total_unstake_ratio =
            Decimal::from_ratio(entry.unstake_amount, total_liquid_stake);

        let unstake_return_native_amount =
            (user_to_total_unstake_ratio * total_received_amount_in_decimal).to_uint_floor();

        entry.unstake_return_native_amount = Some(unstake_return_native_amount);

        record.released = true;

        record.released_height = block_height;

        unbond_record().save(storage, record.id, &record)?;

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
        let staker_undelegation = match staker_undelegation.get_mut(&record.sender.clone()) {
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
) -> Result<(Vec<CosmosMsg>, InjectData), ContractError> {
    if amount < params.min_bond {
        return Err(ContractError::BondAmountTooLow {});
    }

    let validators_reg = crate::state::VALIDATORS_REGISTRY.load(storage)?;

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

    state.total_delegated_amount = delegated_amount;
    let unclaimed_reward = get_unclaimed_reward(
        querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list,
    )?;

    let reward_balance = REWARD_BALANCE.load(storage)?;
    let reward = reward_balance + unclaimed_reward;
    let fee = calc::calc_with_rate(reward, params.fee_rate);
    let total_bond_amount = delegated_amount + reward - fee;

    let mut exchange_rate = state.exchange_rate;

    if !total_bond_amount.is_zero() && !state.total_supply.is_zero() {
        exchange_rate = Decimal::from_ratio(total_bond_amount, state.total_supply);
    }

    if exchange_rate < Decimal::one() {
        return Err(ContractError::InvalidExchangeRate {});
    }

    let prev_exchange_rate = exchange_rate.clone();
    let new_bond_amount = total_bond_amount + amount;
    let new_exchange_rate = if total_bond_amount != Uint128::zero() {
        Decimal::from_ratio(new_bond_amount, state.total_supply)
    } else {
        Decimal::one()
    };

    state.total_bond_amount = new_bond_amount;
    state.total_delegated_amount += amount;
    state.exchange_rate = exchange_rate;
    STATE.save(storage, &state)?;

    let data = InjectData {
        prev_exchange_rate,
        new_exchange_rate,
        total_supply: state.total_supply.clone(),
        reward_balance,
        unclaimed_reward,
        delegated_amount: state.total_delegated_amount,
        total_bond_amount: state.total_bond_amount,
    };

    Ok((msgs, data))
}
