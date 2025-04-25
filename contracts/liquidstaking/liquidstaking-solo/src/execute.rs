use std::str::FromStr;

use crate::error::ContractError;
use crate::event::{
    BatchReceivedEvent, BatchReleasedEvent, BondEvent, ProcessBatchUnbondingEvent,
    ProcessRewardsEvent, ProcessUnbondingEvent, SplitRewardEvent, UpdateConfigEvent,
    UpdateValidatorsEvent,
};
use crate::helpers;
use crate::msg::{
    BondRewardsPayload, Cw20PayloadMsg, ExecuteMsg, ExecuteRewardMsg, RewardMigrateMsg, ZkgmMessage,
};
use crate::query::query_unreleased_unbond_record_from_batch;
use crate::reply::PROCESS_WITHDRAW_REWARD_REPLY_ID;
use crate::state::{
    QuoteToken, Status, Validator, WithdrawReward, CONFIG, PARAMETERS, PENDING_BATCH_ID,
    QUOTE_TOKEN, REWARD_BALANCE, SPLIT_REWARD_QUEUE, STATE, STATUS, SUPPLY_QUEUE,
    VALIDATORS_REGISTRY,
};
use crate::types::ChannelId;
use crate::utils::batch::{batches, BatchStatus};
use crate::utils::calc::calculate_exchange_rate;
use crate::utils::calc::calculate_fee_from_reward;
use crate::utils::delegation::{get_unbonding_ucs03_transfer_cosmos_msg, submit_pending_batch};
use crate::utils::validation::validate_validators;
use crate::utils::{
    self, calc::check_slippage, calc::to_uint128, delegation::get_actual_total_delegated,
    delegation::get_actual_total_reward,
};
use cosmwasm_std::{
    attr, from_json, to_json_binary, Addr, Attribute, BankMsg, Coin, CosmosMsg, Decimal, DepsMut,
    DistributionMsg, Env, MessageInfo, Response, SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20ReceiveMsg;
use unionlabs_primitives::Bytes;

/// process bond call to contract
pub fn bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    slippage: Option<Decimal>,
    expected: Uint128,
) -> Result<Response, ContractError> {
    let status = STATUS.load(deps.storage)?;
    if status.bond_is_paused {
        return Err(ContractError::FunctionalityUnderMaintenance {});
    }

    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom.clone();
    let sender = info.sender;
    let delegator = env.contract.address;

    // coin must have be sent along with transaction and it should be in underlying coin denom
    if info.funds.len() > 1usize {
        return Err(ContractError::InvalidAsset {});
    }

    let payment = Coin {
        amount: info
            .funds
            .iter()
            .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
            .ok_or_else(|| ContractError::NoAsset {})?
            .amount
            .clone(),
        denom: coin_denom.clone(),
    };

    let slippage_rate = match slippage {
        Some(rate) => rate,
        None => Decimal::from_str("0.01").unwrap(),
    };

    let (msgs, sub_msgs, bond_data) = utils::delegation::process_bond(
        deps.storage,
        deps.querier,
        sender.to_string(),
        sender.to_string(),
        delegator.clone(),
        payment.amount,
        env.block.time.nanos(),
        params,
        validators_reg.clone(),
        "".to_string(),
        None,
        env.block.height,
    )?;

    check_slippage(bond_data.mint_amount, expected, slippage_rate)?;

    // create bond event here
    let bond_event = BondEvent(
        sender.to_string(),
        sender.to_string(),
        payment.amount.clone(),
        bond_data.delegated_amount.clone(),
        bond_data.mint_amount,
        bond_data.total_bond_amount.clone(),
        bond_data.total_supply,
        bond_data.exchange_rate,
        "".to_string(),
        env.block.time,
        coin_denom.clone(),
    );

    if bond_data.mint_amount == Uint128::zero() {
        return Err(ContractError::InvalidMintAmount {});
    }

    let res: Response = Response::new()
        .add_messages(msgs)
        .add_submessages(sub_msgs)
        .add_event(bond_event)
        .add_attributes(vec![
            attr("action", "bond"),
            attr("from", sender.clone()),
            attr("staker", sender),
            attr("channel_id", "".to_string()),
            attr("bond_amount", payment.amount.to_string()),
            attr("denom", coin_denom.to_string()),
            attr("minted", bond_data.mint_amount),
            attr("exchange_rate", bond_data.exchange_rate.to_string()),
        ]);

    Ok(res)
}

/// Process zkgm unbond callback by calling process_unbond
pub fn zkgm_unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: ChannelId,
    staker: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let status = STATUS.load(deps.storage)?;
    if status.unbond_is_paused {
        return Err(ContractError::FunctionalityUnderMaintenance {});
    }

    let params = PARAMETERS.load(deps.storage)?;

    let sender = info.sender.clone();
    let delegator = env.contract.address.clone();

    let msg = cw20::Cw20QueryMsg::Balance {
        address: delegator.to_string(),
    };
    let balance: cw20::BalanceResponse = deps
        .querier
        .query_wasm_smart(params.cw20_address.clone(), &msg)?;

    if balance.balance < amount {
        return Err(ContractError::NotEnoughAvailableFund {});
    }

    let unstake_request_event = utils::delegation::unstake_request_in_batch(
        env.clone(),
        deps.storage,
        sender.to_string(),
        staker.clone(),
        amount,
        Some(channel_id.raw()),
    )?;

    let res: Response = Response::new().add_event(unstake_request_event);

    Ok(res)
}

/// Process zkgm bond callback by calling process_bond
pub fn zkgm_bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: ChannelId,
    staker: String,
    amount: Uint128,
    salt: String,
    slippage: Option<Decimal>,
    expected: Uint128,
) -> Result<Response, ContractError> {
    let status = STATUS.load(deps.storage)?;
    if status.bond_is_paused {
        return Err(ContractError::FunctionalityUnderMaintenance {});
    }

    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom.clone();
    let sender = info.sender;
    let delegator = env.contract.address;

    let slippage_rate = match slippage {
        Some(rate) => rate,
        None => Decimal::from_str("0.01").unwrap(),
    };

    let (msgs, sub_msgs, bond_data) = utils::delegation::process_bond(
        deps.storage,
        deps.querier,
        sender.to_string(),
        staker.clone(),
        delegator.clone(),
        amount,
        env.block.time.nanos(),
        params,
        validators_reg.clone(),
        salt,
        Some(channel_id.raw()),
        env.block.height,
    )?;

    if bond_data.mint_amount == Uint128::zero() {
        return Err(ContractError::InvalidMintAmount {});
    }

    // create bond event here
    let bond_event = BondEvent(
        sender.to_string(),
        staker.clone(),
        amount.clone(),
        bond_data.delegated_amount.clone(),
        bond_data.mint_amount,
        bond_data.total_bond_amount.clone(),
        bond_data.total_supply,
        bond_data.exchange_rate,
        format!("{}", channel_id),
        env.block.time,
        coin_denom.clone(),
    );
    check_slippage(bond_data.mint_amount, expected, slippage_rate)?;

    let res: Response = Response::new()
        .add_messages(msgs)
        .add_submessages(sub_msgs)
        .add_event(bond_event)
        .add_attributes(vec![
            attr("action", "bond"),
            attr("from", sender),
            attr("staker", staker.to_string()),
            attr("channel_id", format!("{}", channel_id)),
            attr("bond_amount", amount.to_string()),
            attr("denom", coin_denom.to_string()),
            attr("minted", bond_data.mint_amount),
            attr("exchange_rate", bond_data.exchange_rate.to_string()),
        ]);

    Ok(res)
}

/// Process receive msg from liquid stoken cw20 contract with embedded unbond payload msg to do unbond/unstake
pub fn receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let status = STATUS.load(deps.storage)?;
    if status.unbond_is_paused {
        return Err(ContractError::FunctionalityUnderMaintenance {});
    }

    let params = PARAMETERS.load(deps.storage)?;
    // make sure only cw20 contract can call this function
    if info.sender != params.cw20_address {
        return Err(ContractError::Unauthorized {});
    }

    let state = STATE.load(deps.storage)?;
    if state.exchange_rate < Decimal::one() {
        return Err(ContractError::InvalidExchangeRate {});
    }

    let sender = cw20_msg.sender.to_string();
    let the_staker: String = sender.clone();
    let delegator = env.contract.address.clone();

    // make sure the sender is the cw20 contract
    if info.sender != params.cw20_address {
        return Err(ContractError::Unauthorized {});
    }

    let payload_msg: Cw20PayloadMsg = from_json(cw20_msg.msg)?;

    // make sure the payload is Unstake
    if !matches!(payload_msg, Cw20PayloadMsg::Unstake {}) {
        return Err(ContractError::InvalidPayload {});
    }

    let unbond_amount = cw20_msg.amount;

    let msg = cw20::Cw20QueryMsg::Balance {
        address: delegator.to_string(),
    };

    let balance: cw20::BalanceResponse = deps
        .querier
        .query_wasm_smart(params.cw20_address.clone(), &msg)?;

    if balance.balance < unbond_amount {
        return Err(ContractError::NotEnoughAvailableFund {});
    }

    let unstake_request_event = utils::delegation::unstake_request_in_batch(
        env.clone(),
        deps.storage,
        sender.to_string(),
        the_staker.clone(),
        unbond_amount,
        None,
    )?;

    let res: Response = Response::new().add_event(unstake_request_event);

    Ok(res)
}

/// Process pending batch and execute it
pub fn submit_batch(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let params = PARAMETERS.load(deps.storage)?;
    let delegator = env.contract.address.clone();
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    // first load pending batch
    let pending_batch_id = PENDING_BATCH_ID.load(deps.storage)?;
    let mut pending_batch = batches().load(deps.storage, pending_batch_id)?;

    // make sure the batch execution time is correct
    if let Some(est_next_batch_time) = pending_batch.next_batch_action_time {
        // Check if the batch has been submitted
        if env.block.time.seconds() < est_next_batch_time {
            return Err(ContractError::BatchNotReady {
                actual: env.block.time.seconds(),
                expected: est_next_batch_time,
            });
        }
    } else {
        // Should not enter as pending batch should have a next batch action time
        return Err(ContractError::BatchNotReady {
            actual: env.block.time.seconds(),
            expected: 0u64,
        });
    }

    let (msgs, events) = submit_pending_batch(
        deps,
        env.block.height,
        env.block.time,
        info.sender,
        delegator.clone(),
        &mut pending_batch,
        params,
        validators_reg.clone(),
    )?;

    let res: Response = Response::new().add_messages(msgs).add_events(events);

    Ok(res)
}

// Set the batch received amount and set the batch status to received
// This will be called by backend and the amount data is pulled from indexer when batch complete unbonding is already executed on chain
pub fn set_batch_received_amount(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u64,
    amount: Uint128,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut batch = batches().load(deps.storage, id)?;

    if batch.status != BatchStatus::Submitted {
        // Should not enter as pending batch should have a next batch action time
        return Err(ContractError::BatchStatusIncorrect {
            actual: batch.status,
            expected: BatchStatus::Submitted,
        });
    }
    if env.block.time.seconds() < batch.next_batch_action_time.unwrap() {
        return Err(ContractError::BatchNotReady {
            actual: env.block.time.seconds(),
            expected: batch.next_batch_action_time.unwrap(),
        });
    }

    if amount > batch.expected_native_unstaked.unwrap() || amount == Uint128::zero() {
        return Err(ContractError::InvalidBatchReceivedAmount {});
    }

    batch.update_status(BatchStatus::Received, None);
    batch.received_native_unstaked = Some(amount);
    batches().save(deps.storage, id, &batch)?;

    let event = BatchReceivedEvent(batch.id, amount.to_string(), env.block.time);

    let res: Response = Response::new().add_event(event);

    Ok(res)
}

/// Redelegate some amount that is called from reward contract as result of split reward call to reward contract
pub fn redelegate(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;

    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom.clone();

    // make sure sender is the rewards contract
    if params.reward_address != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let delegator = env.contract.address.clone();

    if info.funds.len() > 1usize {
        return Err(ContractError::InvalidAsset {});
    }

    let payment = Coin {
        amount: info
            .funds
            .iter()
            .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
            .ok_or_else(|| ContractError::NoAsset {})?
            .amount
            .clone(),
        denom: coin_denom.clone(),
    };

    let msgs = utils::delegation::get_delegate_to_validator_msgs(
        delegator.to_string(),
        payment.amount,
        params.underlying_coin_denom.to_string(),
        validators_reg.validators.clone(),
    );

    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    // logic to mint token and update the supply and total_bond_amount
    let mut state = STATE.load(deps.storage)?;

    let delegated_amount = get_actual_total_delegated(
        deps.querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list.clone(),
    )?;

    state.total_delegated_amount = delegated_amount;
    let total_reward = get_actual_total_reward(
        deps.storage,
        deps.querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list,
    )?;

    let fee = calculate_fee_from_reward(total_reward, params.fee_rate);
    let total_bond_amount = delegated_amount + total_reward - fee;

    // after update exchange rate we update the state
    state.total_bond_amount = total_bond_amount + payment.amount;
    state.total_delegated_amount += payment.amount;
    state.last_bond_time = env.block.time.nanos();
    let supply_queue = SUPPLY_QUEUE.load(deps.storage)?;
    state.exchange_rate =
        calculate_exchange_rate(state.total_bond_amount, state.total_supply, &supply_queue);

    STATE.save(deps.storage, &state)?;

    let res: Response = Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "redelegate"),
        attr("from", info.sender.to_string()),
        attr("payment_amount", payment.amount.to_string()),
        attr("denom", coin_denom.to_string()),
        attr("exchange_rate", state.exchange_rate.to_string()),
    ]);

    Ok(res)
}

/// Process rewards by withdraw delegator reward then call redelegate to reward contract on reply
pub fn process_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let status = STATUS.load(deps.storage)?;
    if status.unbond_is_paused {
        return Err(ContractError::FunctionalityUnderMaintenance {});
    }

    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom;
    let sender = info.sender;
    let delegator = env.contract.address;
    let mut sub_msgs: Vec<SubMsg> = vec![];

    let mut attrs = vec![attr("action", "process_rewards"), attr("from", sender)];

    let mut total_amount: Uint128 = Uint128::zero();

    for validator in validators_reg.validators {
        let delegation_rewards = deps
            .querier
            .query_delegation_rewards(delegator.clone(), validator.address.to_string())?;

        let mut payload = BondRewardsPayload {
            validator: validator.address.clone(),
            amount: Uint128::zero(),
        };

        for reward in delegation_rewards {
            if reward.denom == coin_denom {
                payload.amount = to_uint128(reward.amount.to_uint_floor())?;
                total_amount += payload.amount;
            }
        }

        let withdraw_reward_msg: CosmosMsg =
            CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                validator: validator.address.to_string(),
            });

        if payload.amount != Uint128::zero() {
            let payload_bin = to_json_binary(&payload)?;

            let sub_msg: SubMsg =
                SubMsg::reply_always(withdraw_reward_msg, PROCESS_WITHDRAW_REWARD_REPLY_ID)
                    .with_payload(payload_bin)
                    .into();
            sub_msgs.push(sub_msg);
        }
        attrs.push(attr("amount", payload.amount.to_string()));
    }

    SPLIT_REWARD_QUEUE.save(
        deps.storage,
        &WithdrawReward {
            target_amount: total_amount,
            withdrawed_amount: Uint128::zero(),
        },
    )?;

    let ev = ProcessRewardsEvent(total_amount);
    let res: Response = Response::new()
        .add_attributes(attrs)
        .add_event(ev)
        .add_submessages(sub_msgs);

    Ok(res)
}

/// Update the ownership of the contract.
#[allow(clippy::needless_pass_by_value)]
pub fn update_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    if action == cw_ownable::Action::RenounceOwnership {
        return Err(ContractError::OwnershipCannotBeRenounced);
    };

    cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;

    let res: Response = Response::new().add_attribute("action", "update_ownership");

    Ok(res)
}

/// Set contract parameters
pub fn set_parameters(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    underlying_coin_denom: Option<String>,
    liquidstaking_denom: Option<String>,
    ucs03_relay_contract: Option<String>,
    unbonding_time: Option<u64>,
    cw20_address: Option<Addr>,
    reward_address: Option<Addr>,
    fee_receiver: Option<Addr>,
    fee_rate: Option<Decimal>,
    batch_period: Option<u64>,
    epoch_period: Option<u32>,
    min_bond: Option<Uint128>,
    min_unbond: Option<Uint128>,
    batch_limit: Option<u32>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if fee_rate.is_some() && fee_rate.unwrap() > Decimal::one() {
        return Err(ContractError::InvalidFeeRate {});
    }

    let mut params = PARAMETERS.load(deps.storage)?;

    params.underlying_coin_denom = underlying_coin_denom
        .clone()
        .unwrap_or_else(|| params.underlying_coin_denom);
    params.liquidstaking_denom = liquidstaking_denom
        .clone()
        .unwrap_or_else(|| params.liquidstaking_denom);
    params.ucs03_relay_contract = ucs03_relay_contract
        .clone()
        .unwrap_or_else(|| params.ucs03_relay_contract);
    params.unbonding_time = unbonding_time
        .clone()
        .unwrap_or_else(|| params.unbonding_time);
    params.cw20_address = cw20_address.clone().unwrap_or_else(|| params.cw20_address);
    params.reward_address = reward_address
        .clone()
        .unwrap_or_else(|| params.reward_address);

    params.fee_receiver = fee_receiver.clone().unwrap_or_else(|| params.fee_receiver);
    params.fee_rate = fee_rate.clone().unwrap_or_else(|| params.fee_rate);
    params.min_bond = min_bond.clone().unwrap_or_else(|| params.min_bond);
    params.min_unbond = min_unbond.clone().unwrap_or_else(|| params.min_unbond);
    params.batch_limit = batch_limit.clone().unwrap_or_else(|| params.batch_limit);

    if batch_period.is_some() {
        params.batch_period = batch_period.unwrap();
    };

    // update epoch period in SUPPLY QUEUE
    if epoch_period.is_some() {
        let mut supply_queue = SUPPLY_QUEUE.load(deps.storage)?;
        supply_queue.epoch_period = epoch_period.unwrap();
        SUPPLY_QUEUE.save(deps.storage, &supply_queue)?;
    }

    let cw20_addr_string = match cw20_address {
        Some(cw20) => cw20.to_string(),
        None => "".to_string(),
    };
    let mut reward_address_str = "".to_string();

    let mut msgs: Vec<CosmosMsg> = vec![];

    if reward_address.is_some() {
        let msg: CosmosMsg = CosmosMsg::Distribution(DistributionMsg::SetWithdrawAddress {
            address: reward_address.clone().unwrap().to_string(),
        });
        msgs.push(msg);
        reward_address_str = reward_address.unwrap().to_string();
    }
    PARAMETERS.save(deps.storage, &params)?;

    // change the fee receiver and fee rate on reward contract
    if fee_receiver.is_some() || fee_rate.is_some() {
        let msg = ExecuteRewardMsg::SetConfig {
            fee_receiver,
            fee_rate,
            lst_contract_address: None,
            coin_denom: None,
        };
        let msg_bin = to_json_binary(&msg)?;
        let msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: params.reward_address.to_string(),
            msg: msg_bin,
            funds: vec![],
        });
        msgs.push(msg);
    }

    let res: Response = Response::new()
        .add_messages(msgs)
        .add_attribute("action", "set_parameters")
        .add_attribute(
            "liquidstaking_denom",
            liquidstaking_denom.unwrap_or_else(|| "".to_string()),
        )
        .add_attribute(
            "underlying_coin_denom",
            underlying_coin_denom.unwrap_or_else(|| "".to_string()),
        )
        .add_attribute(
            "ucs03_relay_contract",
            ucs03_relay_contract.unwrap_or_else(|| "".to_string()),
        )
        .add_attribute("cw20_address", cw20_addr_string)
        .add_attribute("reward_address", reward_address_str);

    Ok(res)
}

#[derive(Debug)]
pub struct StakerUndelegation {
    pub unstake_amount: Uint128,
    pub channel_id: Option<u32>,
    pub unstake_return_native_amount: Option<Uint128>,
}

/// Process received batch to release the native token back to user so user doesn't need to manually withdraw token
/// 1. Get all unbonding records from pending batch
/// 2. Get how much every user unstaked_native_amount result base on ratio of the user unstaked token to total liquid staked on current batch
/// 3. Set unbond records to released and set released height
/// 4. Generate cosmos msg to send token back to user
pub fn process_batch_withdrawal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u64,
    salt: Vec<String>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let params = PARAMETERS.load(deps.storage)?;
    let mut batch = batches().load(deps.storage, id)?;

    if batch.status != BatchStatus::Received {
        return Err(ContractError::BatchStatusIncorrect {
            actual: batch.status,
            expected: BatchStatus::Received,
        });
    }

    if batch.received_native_unstaked.is_none() {
        return Err(ContractError::BatchIncompleteUnbonding {});
    }

    let total_received_amount = batch.received_native_unstaked.unwrap();

    let mut unbonding_records =
        query_unreleased_unbond_record_from_batch(deps.storage, batch.id, params.batch_limit);

    let is_last_query = if unbonding_records.len() < params.batch_limit as usize {
        true
    } else {
        false
    };

    let (staker_undelegation, unbond_record_ids, total_released_amount) =
        crate::utils::delegation::get_staker_undelegation(
            deps.storage,
            total_received_amount,
            &mut unbonding_records,
            batch.total_liquid_stake,
            env.block.height,
        )?;

    let time = env.block.time;
    let denom = params.underlying_coin_denom;
    let ucs03_relay_contract = params.ucs03_relay_contract;
    let lst_contract = env.contract.address;

    let mut events = vec![];

    let mut send_msgs: Vec<CosmosMsg> = vec![];
    let mut i = 0;
    let transfer_handler = params.transfer_handler.clone();
    for (key, undelegation) in staker_undelegation.iter() {
        // if staker from same chain, we send bank transfer directly
        if undelegation.channel_id.is_none() {
            let bank_msg = BankMsg::Send {
                to_address: key.clone(),
                amount: vec![Coin {
                    denom: denom.clone(),
                    amount: undelegation.unstake_return_native_amount.unwrap(),
                }],
            };
            let msg: CosmosMsg = CosmosMsg::Bank(bank_msg);
            send_msgs.push(msg);
        } else {
            // if staker is from other chain
            // need to send transfer to transfer handler first before transfer to other chain via ucs03 contract
            let bank_msg = BankMsg::Send {
                to_address: transfer_handler.clone(),
                amount: vec![Coin {
                    denom: denom.clone(),
                    amount: undelegation.unstake_return_native_amount.unwrap(),
                }],
            };
            let msg: CosmosMsg = CosmosMsg::Bank(bank_msg);
            send_msgs.push(msg);

            //after send bank msg to transfer handler, then call ucs03 on behalf of transfer handler to send token back
            let msg = get_unbonding_ucs03_transfer_cosmos_msg(
                deps.storage,
                lst_contract.clone(),
                key.clone(),
                undelegation.channel_id,
                time,
                ucs03_relay_contract.clone(),
                undelegation.unstake_return_native_amount.unwrap(),
                params.transfer_fee,
                denom.clone(),
                salt.get(i).unwrap().clone(),
            )?;
            send_msgs.push(msg);
        }

        let ev = ProcessUnbondingEvent(
            id,
            undelegation.channel_id,
            key.to_string(),
            undelegation.unstake_return_native_amount.unwrap(),
            denom.clone(),
            env.block.time,
        );
        events.push(ev);

        i += 1;
    }

    if total_released_amount > Uint128::zero() {
        let ev = ProcessBatchUnbondingEvent(
            id,
            time,
            total_released_amount,
            batch.received_native_unstaked.unwrap(),
            denom.clone(),
            unbond_record_ids,
        );

        events.push(ev);
    }

    if is_last_query {
        batch.update_status(utils::batch::BatchStatus::Released, None);
        batches().save(deps.storage, id, &batch)?;
        let ev = BatchReleasedEvent(batch.id, env.block.time);
        events.push(ev);
    }

    let res: Response = Response::new()
        .add_messages(send_msgs)
        .add_events(events)
        .add_attribute("action", "process_batch_withdrawal")
        .add_attribute("batch_id", batch.id.to_string());

    Ok(res)
}

/// Zkgm callback function to process bond and unbond from another chain
pub fn on_zkgm(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: ChannelId,
    sender: Bytes,
    message: Bytes,
) -> Result<Response, ContractError> {
    let msg_bytes = message.as_ref();
    let payload: ZkgmMessage = from_json(msg_bytes)?;
    let msg = format!(
        "on zgkm time:{} info sender :{}, channel_id:{}, source sender:{} payload:{:?}",
        env.block.time,
        info.sender.to_string(),
        channel_id,
        sender,
        payload
    );
    deps.api.debug(&msg);

    // only ucs03 relayer contract can call this callback function
    let params = PARAMETERS.load(deps.storage)?;
    if info.sender.to_string() != params.ucs03_relay_contract {
        return Err(ContractError::Unauthorized {});
    }

    match payload {
        ZkgmMessage::Bond {
            amount,
            salt,
            slippage,
            expected,
        } => {
            return zkgm_bond(
                deps,
                env,
                info,
                channel_id,
                format!("{}", sender),
                amount,
                salt,
                slippage,
                expected,
            )
        }
        ZkgmMessage::Unbond { amount } => {
            return zkgm_unbond(deps, env, info, channel_id, format!("{}", sender), amount)
        }
    }
}

/// Update the ownership of the contract.
#[allow(clippy::needless_pass_by_value)]
pub fn update_validators(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    validators: Vec<Validator>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if validators.len() < 1 {
        return Err(ContractError::EmptyValidator {});
    }

    validate_validators(&validators)?;

    let mut validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let prev_validators = validators_reg.validators.clone();
    validators_reg.validators = validators.clone();
    VALIDATORS_REGISTRY.save(deps.storage, &validators_reg)?;

    let msgs: Vec<CosmosMsg> = utils::delegation::adjust_validators_delegation(
        deps,
        env.contract.address,
        prev_validators.clone(),
        validators.clone(),
    )?;

    let event = UpdateValidatorsEvent(info.sender.to_string(), prev_validators, validators);
    let res: Response = Response::new().add_messages(msgs).add_event(event);
    Ok(res)
}

/// Update the quote token of the contract for specific channel_id
#[allow(clippy::needless_pass_by_value)]
pub fn update_quote_token(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    channel_id: u32,
    quote_token: QuoteToken,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    if channel_id != quote_token.channel_id {
        return Err(ContractError::InvalidQuoteTokens {});
    }
    QUOTE_TOKEN.save(deps.storage, channel_id, &quote_token)?;
    Ok(Response::default())
}

pub fn set_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    lst_contract_address: Option<Addr>,
    fee_receiver: Option<Addr>,
    fee_rate: Option<Decimal>,
    coin_denom: Option<String>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let mut config = CONFIG.load(deps.storage)?;

    if fee_rate.is_some() && fee_rate.unwrap() > Decimal::one() {
        return Err(ContractError::InvalidFeeRate {});
    }

    config.lst_contract_address = lst_contract_address
        .clone()
        .unwrap_or_else(|| config.lst_contract_address);
    config.fee_receiver = fee_receiver.clone().unwrap_or_else(|| config.fee_receiver);
    config.fee_rate = fee_rate.clone().unwrap_or_else(|| config.fee_rate);
    config.coin_denom = coin_denom.clone().unwrap_or_else(|| config.coin_denom);
    CONFIG.save(deps.storage, &config)?;

    let event = UpdateConfigEvent(
        config.lst_contract_address.clone(),
        config.fee_receiver.clone(),
        config.fee_rate,
        config.coin_denom.clone(),
    );

    let attrs = Vec::from([
        attr("action", "set_config"),
        attr("lst_contract_address", config.lst_contract_address),
        attr("fee_receiver", config.fee_receiver),
        attr("fee_rate", config.fee_rate.to_string()),
        attr("coin_denom", config.coin_denom),
    ]);

    Ok(Response::new().add_attributes(attrs).add_event(event))
}

/// Migrate reward contract
pub fn migrate_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    code_id: u64,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let params = PARAMETERS.load(deps.storage)?;

    if params.reward_address == env.contract.address {
        return Err(ContractError::InvalidRewardContractMigration {});
    }

    let migrate = RewardMigrateMsg {};
    let msg_bin = to_json_binary(&migrate)?;
    let migrate_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Migrate {
        contract_addr: params.reward_address.to_string(),
        new_code_id: code_id,
        msg: msg_bin,
    });

    let res: Response = Response::new().add_message(migrate_msg);
    Ok(res)
}

pub fn split_reward(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // only liquid staking contract able to call this function
    if info.sender != config.lst_contract_address {
        return Err(ContractError::Unauthorized {});
    }

    // first need to get this contract balance
    let contract_addr: Addr = env.contract.address;
    let balance = deps
        .querier
        .query_balance(contract_addr, config.coin_denom.clone())?;

    let mut msgs: Vec<CosmosMsg> = vec![];

    if balance.amount == Uint128::zero() {
        return Err(ContractError::NotEnoughFund {});
    }

    // check total balance from reward
    let mut reward_balance = REWARD_BALANCE.load(deps.storage)?;

    let mut balance_to_split = reward_balance;

    if balance.amount < balance_to_split {
        balance_to_split = balance.amount;
    }

    //reset the reward balance
    reward_balance = Uint128::zero();
    REWARD_BALANCE.save(deps.storage, &reward_balance)?;

    let mut attrs: Vec<Attribute> = vec![
        attr("action", "split_reward"),
        attr("fee_rate", format!("{:?}", config.fee_rate)),
        attr("amount", balance.amount.to_string()),
        attr("fee_receiver", config.fee_receiver.to_string()),
        attr("time", format!("{}", env.block.time.nanos())),
    ];
    let (redelegate, fee) =
        helpers::split_revenue(balance_to_split, config.fee_rate, config.coin_denom);

    // Send the fee to revenue receiver
    let bank_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: config.fee_receiver.to_string(),
        amount: vec![fee.clone()],
    });

    msgs.push(bank_msg);

    // Redelegate by call the LST Contract and attach the funds
    let lst: helpers::LstTemplateContract =
        helpers::LstTemplateContract(config.lst_contract_address);
    let execute_msg = lst.call(ExecuteMsg::Redelegate {}, vec![redelegate.clone()])?;
    msgs.push(execute_msg);

    attrs.push(attr("redelegate_amount", redelegate.amount.to_string()));
    attrs.push(attr("fee_amount", fee.amount.to_string()));

    let event = SplitRewardEvent(
        config.fee_rate,
        balance_to_split,
        redelegate.amount,
        fee.amount,
        env.block.time,
    );

    // transfer the fee to revenue receiver
    Ok(Response::new()
        .add_messages(msgs)
        .add_event(event)
        .add_attributes(attrs))
}

pub fn set_status(
    deps: DepsMut,
    info: MessageInfo,
    status: Status,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    STATUS.save(deps.storage, &status)?;
    Ok(Response::new())
}
