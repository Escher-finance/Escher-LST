use crate::error::ContractError;
use crate::event::{
    BatchReceivedEvent, BatchReleasedEvent, BondEvent, ProcessBatchUnbondingEvent,
    ProcessRewardsEvent, ProcessUnbondingEvent, UpdateValidatorsEvent,
};
use crate::msg::{BondRewardsPayload, Cw20PayloadMsg, ExecuteRewardMsg, MigrateMsg, ZkgmMessage};
use crate::query::query_unreleased_unbond_record_from_batch;
use crate::reply::PROCESS_WITHDRAW_REWARD_REPLY_ID;
use crate::state::{
    Chain, QuoteToken, Status, Validator, WithdrawReward, PARAMETERS, PENDING_BATCH_ID,
    QUOTE_TOKEN, REWARD_BALANCE, SPLIT_REWARD_QUEUE, STATE, STATUS, VALIDATORS_REGISTRY,
};
use crate::types::ChannelId;
use crate::utils::batch::{batches, BatchStatus};
use crate::utils::calc::to_uint128;
use crate::utils::validation::{validate_recipient, validate_validators};
use crate::utils::{
    self, delegation::get_actual_total_delegated, delegation::get_mock_total_reward,
    delegation::get_unbonding_ucs03_transfer_cosmos_msg, delegation::get_unclaimed_reward,
    delegation::submit_pending_batch,
};
use crate::zkgm::com::ZkgmHubMsg;
use crate::zkgm::protocol::{get_hub_ack_msg, ucs03_transfer_and_call};
use alloy::primitives::U256;
use cosmwasm_std::{
    attr, from_json, to_json_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut,
    DistributionMsg, Env, MessageInfo, Response, SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20ReceiveMsg;
use unionlabs_primitives::Bytes;

/// process bond/stake to contract
pub fn bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _slippage: Option<Decimal>,
    _expected: Uint128,
    recipient: Option<String>,
    recipient_channel_id: Option<u32>,
    salt: Option<String>,
) -> Result<Response, ContractError> {
    let status = STATUS.load(deps.storage)?;
    if status.bond_is_paused {
        return Err(ContractError::FunctionalityUnderMaintenance {});
    }

    let on_chain_recipient =
        validate_recipient(&deps, recipient.clone(), recipient_channel_id, salt.clone())?;

    // coin must have be sent along with transaction and it should be in underlying coin denom
    if info.funds.len() > 1usize {
        return Err(ContractError::InvalidAsset {});
    }

    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom.clone();
    let sender = info.sender;
    let delegator = env.contract.address;

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

    // get cosmos messages to delegate and mint liquid staking token
    let (msgs, sub_msgs, bond_data) = utils::delegation::process_bond(
        deps.storage,
        deps.querier,
        sender.to_string(),
        sender.to_string(),
        delegator.clone(),
        payment.amount,
        payment.amount,
        env.block.time.nanos(),
        params,
        validators_reg.clone(),
        salt.unwrap_or("".into()),
        None,
        env.block.height,
        recipient.clone(),
        recipient_channel_id,
        on_chain_recipient,
    )?;

    // create bond event here
    // create bond event here
    let bond_event = BondEvent(
        1,
        sender.to_string(),
        payment.amount.clone(),
        bond_data.delegated_amount.clone(),
        bond_data.lst_amount,
        bond_data.total_bond_amount.clone(),
        bond_data.total_supply,
        bond_data.exchange_rate,
        "".to_string(),
        env.block.time.nanos(),
        bond_data.reward_balance,
        bond_data.unclaimed_reward,
    );

    if bond_data.lst_amount == Uint128::zero() {
        return Err(ContractError::InvalidMintAmount {});
    }

    let res: Response = Response::new()
        .add_messages(msgs)
        .add_submessages(sub_msgs)
        .add_event(bond_event)
        .add_attributes(vec![
            attr("action", "bond"),
            attr("from", sender.to_string()),
            attr("staker", sender),
            attr("channel_id", "".to_string()),
            attr("bond_amount", payment.amount.to_string()),
            attr("denom", coin_denom),
            attr("lst_amount", bond_data.lst_amount),
            attr("exchange_rate", bond_data.exchange_rate.to_string()),
        ]);

    Ok(res)
}

/// Process zkgm unbond callback to process delegate and undelegate
pub fn zkgm_hub_batch(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: u32,
    hub_batch_id: u32,
    delegate_amount: Uint128,
    unstake_amount: Uint128,
    mint_amount: Uint128,
    salt: String,
) -> Result<Response, ContractError> {
    let status = STATUS.load(deps.storage)?;
    if status.unbond_is_paused {
        return Err(ContractError::FunctionalityUnderMaintenance {});
    }

    let params = PARAMETERS.load(deps.storage)?;

    let balance = deps.querier.query_balance(
        env.contract.address.clone(),
        params.underlying_coin_denom.clone(),
    )?;

    if balance.amount < delegate_amount {
        return Err(ContractError::NotEnoughAvailableFund {});
    }

    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;

    // make sure channel id is correct, it must be equals to the hub channel id
    let delegator = env.contract.address.clone();
    // stake the amount part of the batch

    let (mut msgs, bond_event, exchange_rate) = utils::delegation::process_staking_batch(
        deps.storage,
        deps.querier,
        channel_id,
        info.sender.to_string(),
        hub_batch_id,
        delegator.clone(),
        delegate_amount,
        mint_amount,
        env.block.time.nanos(),
        params.clone(),
        validators_reg,
    )?;

    let unstake_request_event = utils::delegation::unstake_request(
        env.clone(),
        deps.storage,
        hub_batch_id,
        params.hub_contract.clone(),
        unstake_amount,
        Some(params.hub_channel_id),
    )?;

    // send ack message to hub contract with latest exchange rate

    let payload = ZkgmHubMsg {
        action: "hub_batch_ack".into(),
        id: hub_batch_id,
        amount: U256::from(0u128),
        rate: U256::from(exchange_rate.atomics().u128()),
        union_block: env.block.height,
    };

    let msg = get_hub_ack_msg(
        delegator.to_string(),
        params.ucs03_relay_contract,
        channel_id,
        env.block.time,
        params.hub_contract,
        payload,
        salt,
    )?;

    msgs.push(msg);

    let res: Response = Response::new()
        .add_event(bond_event)
        .add_event(unstake_request_event)
        .add_messages(msgs);

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

    // make sure the sender is the cw20 contract
    if info.sender != params.cw20_address {
        return Err(ContractError::Unauthorized {});
    }

    let sender = cw20_msg.sender.to_string();
    let the_staker: String = sender.clone();
    let delegator = env.contract.address.clone();

    let payload_msg: Cw20PayloadMsg = from_json(cw20_msg.msg)?;

    // make sure the payload is Unstake
    if !matches!(
        payload_msg,
        Cw20PayloadMsg::Unstake {
            recipient: _,
            recipient_channel_id: _
        }
    ) {
        return Err(ContractError::InvalidPayload {});
    }

    // get the recipient and recipient channel id from payload_msg
    let (recipient, recipient_channel_id) = match payload_msg {
        Cw20PayloadMsg::Unstake {
            recipient,
            recipient_channel_id,
        } => (recipient, recipient_channel_id),
    };

    validate_recipient(
        &deps,
        recipient.clone(),
        recipient_channel_id,
        Some("".into()),
    )?; // salt is not required in unbond request

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
        1,
        sender.to_string(),
        the_staker.clone(),
        unbond_amount,
        None,
        recipient,
        recipient_channel_id,
    )?;

    let res: Response = Response::new().add_event(unstake_request_event);

    Ok(res)
}

/// Process pending batch and execute it
pub fn submit_batch(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    salt: String,
) -> Result<Response, ContractError> {
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

    let (mut msgs, events, exchange_rate) = submit_pending_batch(
        deps,
        env.block.time,
        info.sender,
        delegator.clone(),
        &mut pending_batch,
        params.clone(),
        validators_reg.clone(),
    )?;

    let payload = ZkgmHubMsg {
        action: "hub_batch_unbonding_ack".into(),
        id: pending_batch_id as u32,
        amount: U256::from(pending_batch.total_liquid_stake.u128()),
        rate: U256::from(exchange_rate.atomics().u128()),
        union_block: env.block.height,
    };

    let msg = get_hub_ack_msg(
        delegator.to_string(),
        params.ucs03_relay_contract.clone(),
        params.hub_channel_id,
        env.block.time,
        params.hub_contract,
        payload,
        salt,
    )?;

    msgs.push(msg);

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

    if amount > batch.expected_native_unstaked.unwrap() {
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

    let total_bond_amount: Uint128;
    if !cfg!(test) {
        state.total_delegated_amount = delegated_amount;

        let unclaimed_reward = get_unclaimed_reward(
            deps.querier,
            delegator.to_string(),
            coin_denom.clone(),
            validators_list,
        )?;

        // query the reward from this contract state
        let contract_reward_balance = REWARD_BALANCE.load(deps.storage)?;
        let reward = unclaimed_reward + contract_reward_balance;
        total_bond_amount = delegated_amount + reward;
    } else {
        total_bond_amount = get_mock_total_reward(state.total_bond_amount);
    }

    // after update exchange rate we update the state
    state.total_bond_amount = total_bond_amount + payment.amount;
    state.total_delegated_amount += payment.amount;
    state.last_bond_time = env.block.time.nanos();
    state.update_exchange_rate();

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

        if payload.amount != Uint128::zero() {
            let payload_bin = to_json_binary(&payload)?;

            let withdraw_reward_msg: CosmosMsg =
                CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                    validator: validator.address.to_string(),
                });

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
    underlying_coin_denom_symbol: Option<String>,
    liquidstaking_denom: Option<String>,
    liquidstaking_denom_symbol: Option<String>,
    ucs03_relay_contract: Option<String>,
    unbonding_time: Option<u64>,
    cw20_address: Option<Addr>,
    reward_address: Option<Addr>,
    fee_receiver: Option<Addr>,
    fee_rate: Option<Decimal>,
    batch_period: Option<u64>,
    min_bond: Option<Uint128>,
    min_unbond: Option<Uint128>,
    batch_limit: Option<u32>,
    transfer_handler: Option<String>,
    transfer_fee: Option<Uint128>,
    zkgm_token_minter: Option<String>,
    hub_channel_id: Option<u32>,
    hub_quote_token: Option<String>,
    hub_contract: Option<String>,
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
    params.transfer_handler = transfer_handler
        .clone()
        .unwrap_or_else(|| params.transfer_handler);
    params.transfer_fee = transfer_fee.clone().unwrap_or_else(|| params.transfer_fee);
    params.zkgm_token_minter = zkgm_token_minter
        .clone()
        .unwrap_or_else(|| params.zkgm_token_minter);
    params.underlying_coin_denom_symbol = underlying_coin_denom_symbol
        .clone()
        .unwrap_or_else(|| params.underlying_coin_denom_symbol);

    params.liquidstaking_denom_symbol = liquidstaking_denom_symbol
        .clone()
        .unwrap_or_else(|| params.liquidstaking_denom_symbol);

    params.hub_channel_id = hub_channel_id
        .clone()
        .unwrap_or_else(|| params.hub_channel_id);

    params.hub_quote_token = hub_quote_token
        .clone()
        .unwrap_or_else(|| params.hub_quote_token);

    params.hub_contract = hub_contract.clone().unwrap_or_else(|| params.hub_contract);

    if batch_period.is_some() {
        params.batch_period = batch_period.unwrap();
    };

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

    let transfer_fee_str = match transfer_fee {
        Some(fee) => fee.to_string(),
        None => "".to_string(),
    };
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
        .add_attribute("reward_address", reward_address_str)
        .add_attribute("transfer_fee", transfer_fee_str)
        .add_attribute(
            "transfer_handler",
            transfer_handler.unwrap_or_else(|| "".to_string()),
        )
        .add_attribute(
            "zkgm_token_minter",
            zkgm_token_minter.unwrap_or_else(|| "".to_string()),
        )
        .add_attribute(
            "underlying_coin_denom_symbol",
            underlying_coin_denom_symbol.unwrap_or_else(|| "".to_string()),
        )
        .add_attribute(
            "liquidstaking_denom_symbol",
            liquidstaking_denom_symbol.unwrap_or_else(|| "".to_string()),
        );
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

    let transfer_handler = params.transfer_handler.clone();
    let mut i = 0;
    for (sender, undelegation) in staker_undelegation.iter() {
        let bank_msg = BankMsg::Send {
            to_address: transfer_handler.clone(),
            amount: vec![Coin {
                denom: denom.clone(),
                amount: undelegation.unstake_return_native_amount.unwrap(),
            }],
        };
        let msg: CosmosMsg = CosmosMsg::Bank(bank_msg);
        send_msgs.push(msg);

        let target_channel_id = undelegation.channel_id.unwrap();

        //after send bank msg to transfer handler, then call ucs03 on behalf of transfer handler to send token back
        let msg = get_unbonding_ucs03_transfer_cosmos_msg(
            deps.storage,
            lst_contract.clone(),
            sender.clone(),
            target_channel_id,
            time,
            ucs03_relay_contract.clone(),
            undelegation.unstake_return_native_amount.unwrap(),
            params.transfer_fee,
            denom.clone(),
            salt.get(i).unwrap().clone(),
        )?;
        send_msgs.push(msg);

        let ev = ProcessUnbondingEvent(
            id,
            undelegation.channel_id,
            sender.to_string(),
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
        ZkgmMessage::HubBatch {
            id,
            delegate_amount,
            unstake_amount,
            mint_amount,
            salt,
        } => {
            return zkgm_hub_batch(
                deps,
                env,
                info,
                channel_id.raw(),
                id,
                delegate_amount,
                unstake_amount,
                mint_amount,
                salt,
            );
        }
        ZkgmMessage::SubmitBatch { salt } => return submit_batch(deps, env, info, salt),
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

/// Migrate reward contract
pub fn migrate_reward(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    code_id: u64,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let params = PARAMETERS.load(deps.storage)?;
    let migrate = MigrateMsg {};
    let msg_bin = to_json_binary(&migrate)?;
    let migrate_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Migrate {
        contract_addr: params.reward_address.to_string(),
        new_code_id: code_id,
        msg: msg_bin,
    });

    let res: Response = Response::new().add_message(migrate_msg);
    Ok(res)
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

pub fn set_chain(
    deps: DepsMut,
    info: MessageInfo,
    chain: Chain,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    crate::state::CHAINS.save(deps.storage, chain.ucs03_channel_id, &chain)?;
    Ok(Response::new())
}

pub fn remove_chain(
    deps: DepsMut,
    info: MessageInfo,
    channel_id: u32,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    crate::state::CHAINS.remove(deps.storage, channel_id);
    Ok(Response::new())
}

/// Inject some amount of underlying coin denom to be staked without minting new cw20 token
pub fn inject(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let params = PARAMETERS.load(deps.storage)?;
    let contract_addr: Addr = env.contract.address;
    let balance = deps
        .querier
        .query_balance(contract_addr.clone(), params.underlying_coin_denom.clone())?;

    if balance.amount < amount {
        return Err(ContractError::NotEnoughAvailableFund {});
    }

    let (msgs, inject_data) =
        utils::delegation::inject(deps.storage, deps.querier, contract_addr, amount, params)?;

    let inject_event = crate::event::InjectEvent(
        amount,
        inject_data.reward_balance,
        inject_data.unclaimed_reward,
        inject_data.prev_exchange_rate,
        inject_data.new_exchange_rate,
        inject_data.delegated_amount,
        inject_data.total_bond_amount,
        inject_data.total_supply,
        env.block.time,
    );

    Ok(Response::new()
        .add_messages(msgs)
        .add_event(inject_event)
        .add_attribute("action", "inject")
        .add_attribute("amount", amount))
}

pub fn transfer_and_call(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: u32,
    receiver: String,
    amount: Uint128,
    salt: String,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let params = PARAMETERS.load(deps.storage)?;
    let contract_addr: Addr = env.contract.address.clone();
    let balance = deps
        .querier
        .query_balance(contract_addr.clone(), params.underlying_coin_denom.clone())?;

    if balance.amount < amount {
        return Err(ContractError::NotEnoughAvailableFund {});
    }

    let sender = env.contract.address.to_string();

    let contract_calldata = ZkgmHubMsg {
        action: "hub_batch_ack".into(),
        id: 7,
        amount: U256::from(amount.u128()),
        rate: U256::from(Uint128::one().u128()),
        union_block: env.block.height,
    };

    let msg_bin = ucs03_transfer_and_call(
        env.block.time,
        channel_id,
        sender,
        receiver,
        params.underlying_coin_denom_symbol,
        params.underlying_coin_denom.clone(),
        params.underlying_coin_denom.clone(),
        amount,
        params.hub_quote_token,
        amount,
        salt,
        params.hub_contract,
        contract_calldata,
    )?;

    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: params.ucs03_relay_contract.clone(),
        msg: msg_bin,
        funds: vec![Coin {
            denom: params.underlying_coin_denom.clone(),
            amount,
        }],
    });

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "transfer_and_call")
        .add_attribute("amount", amount))
}
