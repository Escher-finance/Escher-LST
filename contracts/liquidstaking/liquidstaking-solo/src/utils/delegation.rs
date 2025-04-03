use crate::event::{SubmitBatchEvent, UnbondEventsFromAtts, UnstakeRequestEvent};
use crate::msg::ValidatorDelegation;
use crate::proto;
use crate::state::{ValidatorsRegistry, PENDING_BATCH_ID, QUOTE_TOKEN, REWARD_BALANCE};
use crate::utils::{batch::batches, calc, delegation, token};
use crate::ContractError;
use crate::{
    msg::{BondData, DelegationDiff, MintTokensPayload},
    state::{
        increment_tokens, unbond_record, BurnQueue, MintQueue, Parameters, SupplyQueue,
        UnbondRecord, Validator, PARAMETERS, STATE, SUPPLY_QUEUE,
    },
};
use cosmwasm_std::{
    to_json_binary, Addr, Attribute, Binary, CosmosMsg, Decimal, DepsMut, Env, QuerierWrapper,
    StdResult, Storage, SubMsg, Uint128, Uint256,
};
use cosmwasm_std::{AnyMsg, BankMsg, Coin};
use cosmwasm_std::{Event, Timestamp};
use prost::Message;
use std::collections::HashMap;
use std::str::FromStr;
use unionlabs_primitives::{Bytes, H256};

use super::batch::{Batch, BatchStatus};
use super::calc::calculate_fee_from_reward;
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
                    let reward_val = calc::to_uint128(reward.amount.to_uint_floor())?;
                    total_rewards += reward_val;
                }
            }
        }
    }

    Ok(total_rewards)
}

// for testing only
pub fn get_mock_total_reward(total_bond_amount: Uint128) -> Uint128 {
    let ratio = Decimal::from_ratio(Uint128::new(1000), Uint128::new(1005));
    calc::calculate_staking_token_from_rate(total_bond_amount, ratio)
}

pub fn calculate_delegated_amount(amount: Uint128, ratio: Decimal) -> Uint128 {
    (ratio * Decimal::from_ratio(amount, Uint128::one())).to_uint_floor()
}

pub fn get_undelegate_from_validator_msgs(
    delegator: String,
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
    let mut total_undelegated: Uint128 = Uint128::from(0u32);

    for validator in validators.into_iter() {
        let ratio = Decimal::from_ratio(Uint128::from(validator.weight), total_weight);

        let undelegate_amount_dec = Decimal::from_ratio(undelegate_amount, Uint128::one());
        let undelegate_amount_for_validator = (undelegate_amount_dec * ratio).to_uint_floor();

        let undelegate_staking_msg = get_babylon_undelegate_cosmos_msg(
            delegator.to_string(),
            validator.address.to_string(),
            undelegate_amount_for_validator.to_string(),
            coin_denom.clone(),
        );

        msgs.push(undelegate_staking_msg);

        atts.push(Attribute {
            key: validator.address.to_string(),
            value: undelegate_amount_for_validator.to_string(),
        });
        total_undelegated += undelegate_amount_for_validator;
    }

    (total_undelegated, msgs, atts)
}

pub fn get_validator_delegation_map_with_total_bond(
    deps: DepsMut,
    delegator: String,
    validators: Vec<Validator>,
) -> Result<(HashMap<String, Uint128>, Uint128), ContractError> {
    let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    let mut total_delegated_amount = Uint128::from(0u32);

    for validator in validators {
        let validator_bond = deps
            .querier
            .query_delegation(delegator.clone(), validator.address.clone())?;

        let delegation_amount = match validator_bond {
            Some(delegation) => delegation.amount.amount,
            None => Uint128::from(0u32),
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
    delegator: String,
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
                if surplus_validator.diff_amount == Uint128::from(0u32) {
                    break;
                }

                let redelegate_msg = get_babylon_redelegate_cosmos_msg(
                    delegator.clone(),
                    surplus_validator.address.to_string(),
                    deficient_validator.address.clone(),
                    surplus_validator.diff_amount.into(),
                    denom.clone(),
                );
                surplus_validator.diff_amount = Uint128::from(0u32);
                deficient_validator.diff_amount =
                    deficient_validator.diff_amount - surplus_validator.diff_amount;

                msgs.push(redelegate_msg);
            } else {
                let redelegate_msg = get_babylon_redelegate_cosmos_msg(
                    delegator.clone(),
                    surplus_validator.address.to_string(),
                    deficient_validator.address.clone(),
                    deficient_validator.diff_amount.into(),
                    denom.clone(),
                );

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
    delegator: String,
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
    let mut first_validator: String = "".to_string();

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
            validator.address.to_string(),
            delegate_amount.to_string(),
            coin_denom.clone(),
        );

        msgs.push(delegate_msg.into());

        if first_validator.is_empty() {
            first_validator = validator.address.to_string();
        }
    }

    // calculate remaining

    let remaining_amount = delegate_amount - total_delegated;
    if !remaining_amount.is_zero() {
        let delegate_msg = get_babylon_delegate_cosmos_msg(
            delegator.clone(),
            first_validator.to_string(),
            remaining_amount.to_string(),
            coin_denom,
        );

        msgs.push(delegate_msg.into());
    }
    msgs
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

    let msgs: Vec<CosmosMsg> = get_restaking_msgs(
        delegator.to_string(),
        surplus_validators,
        deficient_validators,
        denom,
    );

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
    block_height: u64,
) -> Result<(Vec<CosmosMsg>, Vec<SubMsg>, BondData), ContractError> {
    if amount < params.min_bond {
        return Err(ContractError::BondAmountTooLow {});
    }

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
        coin_denom.clone(),
        validators_list.clone(),
    )?;

    let total_bond_amount: Uint128;
    if !cfg!(test) {
        state.total_delegated_amount = delegated_amount;
        let reward = get_actual_total_reward(
            storage,
            querier,
            delegator.to_string(),
            coin_denom.clone(),
            validators_list,
        )?;

        let fee = calculate_fee_from_reward(reward, params.fee_rate);
        total_bond_amount = delegated_amount + reward - fee;
    } else {
        total_bond_amount = get_mock_total_reward(state.total_bond_amount);
    }

    let mut exchange_rate = state.exchange_rate;
    let mut supply_queue: SupplyQueue = SUPPLY_QUEUE.load(storage)?;
    if total_bond_amount != Uint128::zero() && state.total_supply != Uint128::zero() {
        calc::normalize_supply_queue(&mut supply_queue, block_height);
        exchange_rate =
            calc::calculate_exchange_rate(total_bond_amount, state.total_supply, &supply_queue);
    }

    let mint_amount = calc::calculate_staking_token_from_rate(amount, exchange_rate);

    // after update exchange rate we update the state
    state.bond_counter = state.bond_counter + 1;
    state.total_bond_amount = total_bond_amount + amount;
    state.total_supply += mint_amount;
    state.total_delegated_amount += amount;
    state.last_bond_time = bond_time;
    state.exchange_rate = exchange_rate;

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

    supply_queue.mint.push(MintQueue {
        block: block_height,
        amount: mint_amount,
    });
    SUPPLY_QUEUE.save(storage, &supply_queue)?;

    if !cfg!(test) {
        // Start to mint according to staked token only if it is not test
        let sub_msg: SubMsg = token::get_staked_token_submsg(
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
    block_height: u64,
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

    let fee = calculate_fee_from_reward(reward, params.fee_rate);
    let total_bond_amount = delegated_amount + reward - fee;

    if total_bond_amount.is_zero() || state.total_supply.is_zero() {
        return Err(ContractError::ZeroSupplyOrDelegatedAmount {});
    }

    let mut current_exchange_rate = state.exchange_rate;
    let mut supply_queue: SupplyQueue = SUPPLY_QUEUE.load(storage)?;

    if total_bond_amount != Uint128::zero() && state.total_supply != Uint128::zero() {
        calc::normalize_supply_queue(&mut supply_queue, block_height);
        current_exchange_rate =
            calc::calculate_exchange_rate(total_bond_amount, state.total_supply, &supply_queue);
    }

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
        delegator.to_string(),
        undelegate_amount,
        coin_denom.clone(),
        validators_reg.validators,
    );
    msgs.extend(undelegate_msgs.clone());

    let mut events = UnbondEventsFromAtts(atts, batch.id, time);

    let burn_msg = token::burn_token(batch.total_liquid_stake, params.cw20_address.to_string());
    msgs.push(burn_msg.into());

    supply_queue.burn.push(BurnQueue {
        block: block_height,
        amount: total_undelegate_amount,
    });
    SUPPLY_QUEUE.save(storage, &supply_queue)?;

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

pub fn get_actual_total_reward(
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
    delegator: String,
    denom: String,
    validators: Vec<String>,
) -> StdResult<Uint128> {
    let unclaimed_reward = get_unclaimed_reward(querier, delegator, denom, validators)?;
    let reward_balance = REWARD_BALANCE.load(storage)?;
    Ok(unclaimed_reward + reward_balance)
}

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

    let any_delegate_msg = CosmosMsg::Any(AnyMsg {
        type_url: "/babylon.epoching.v1.MsgWrappedDelegate".to_string(),
        value: Binary::from(staking_msg.encode_to_vec()),
    });
    any_delegate_msg
}

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

    let undelegate_staking_msg = CosmosMsg::Any(AnyMsg {
        type_url: "/babylon.epoching.v1.MsgWrappedUndelegate".to_string(),
        value: Binary::from(wrapped_msg.encode_to_vec()),
    });
    undelegate_staking_msg
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

    let redelegate = CosmosMsg::Any(AnyMsg {
        type_url: "/babylon.epoching.v1.MsgWrappedBeginRedelegate".to_string(),
        value: Binary::from(restaking_msg.encode_to_vec()),
    });
    redelegate
}
