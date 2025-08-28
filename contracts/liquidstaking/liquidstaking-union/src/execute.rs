use cosmwasm_std::{
    attr, from_json, to_json_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut,
    DistributionMsg, Env, MessageInfo, Response, SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20ReceiveMsg;
use unionlabs_primitives::Bytes;

use crate::{
    error::ContractError,
    event::{
        BatchReceivedEvent, BatchReleasedEvent, BondEvent, ProcessBatchUnbondingEvent,
        ProcessRewardsEvent, ProcessUnbondingEvent, UpdateValidatorsEvent,
    },
    msg::{BondRewardsPayload, Cw20PayloadMsg, ExecuteRewardMsg, MigrateMsg, ZkgmMessage},
    query::query_unreleased_unbond_record_from_batch,
    reply::PROCESS_WITHDRAW_REWARD_REPLY_ID,
    state::{
        Chain, QuoteToken, Status, Validator, WithdrawReward, PARAMETERS, PENDING_BATCH_ID,
        QUOTE_TOKEN, REWARD_BALANCE, SPLIT_REWARD_QUEUE, STATE, STATUS, VALIDATORS_REGISTRY,
    },
    types::ChannelId,
    utils::{
        self,
        batch::{batches, BatchStatus},
        calc::{check_slippage, to_uint128},
        delegation::{
            get_actual_total_delegated, get_mock_total_reward,
            get_unbonding_ucs03_transfer_cosmos_msg, get_unclaimed_reward, submit_pending_batch,
        },
        validation::{validate_recipient, validate_validators},
    },
};

/// process bond/stake to contract
#[allow(clippy::too_many_arguments)]
pub fn bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    slippage: Option<Decimal>,
    expected: Uint128,
    recipient: Option<String>,
    recipient_channel_id: Option<u32>,
    salt: Option<String>,
) -> Result<Response, ContractError> {
    let status = STATUS.load(deps.storage)?;
    if status.bond_is_paused {
        return Err(ContractError::FunctionalityUnderMaintenance {});
    }

    let on_chain_recipient = validate_recipient(
        &deps.as_ref(),
        recipient.clone(),
        recipient_channel_id,
        salt.clone(),
    )?;

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
            .amount,
        denom: coin_denom.clone(),
    };

    let slippage_rate = match slippage {
        Some(rate) => rate,
        None => Decimal::from_ratio(Uint128::one(), Uint128::from(100_u32)),
    };

    // get cosmos messages to delegate and mint liquid staking token
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
        salt.unwrap_or("".into()),
        None,
        env.block.height,
        recipient.clone(),
        recipient_channel_id,
        on_chain_recipient,
    )?;

    check_slippage(bond_data.mint_amount, expected, slippage_rate)?;

    // create bond event here
    // create bond event here
    let bond_event = BondEvent(
        sender.to_string(),
        sender.to_string(),
        payment.amount,
        bond_data.delegated_amount,
        bond_data.mint_amount,
        bond_data.total_bond_amount,
        bond_data.total_supply,
        bond_data.exchange_rate,
        "".to_string(),
        env.block.time,
        coin_denom.clone(),
        recipient.clone(),
        recipient_channel_id,
        bond_data.reward_balance,
        bond_data.unclaimed_reward,
        None,
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
            attr("from", sender.to_string()),
            attr("staker", sender),
            attr("channel_id", "".to_string()),
            attr("bond_amount", payment.amount.to_string()),
            attr("denom", coin_denom),
            attr("minted", bond_data.mint_amount),
            attr("exchange_rate", bond_data.exchange_rate.to_string()),
        ]);

    Ok(res)
}

/// Process zkgm unbond callback by calling process_unbond
#[allow(clippy::too_many_arguments)]
pub fn zkgm_unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: ChannelId,
    staker: String,
    amount: Uint128,
    recipient: Option<String>,
    recipient_channel_id: Option<u32>,
) -> Result<Response, ContractError> {
    let status = STATUS.load(deps.storage)?;
    if status.unbond_is_paused {
        return Err(ContractError::FunctionalityUnderMaintenance {});
    }

    validate_recipient(
        &deps.as_ref(),
        recipient.clone(),
        recipient_channel_id,
        Some("".into()),
    )?; // salt is not required in unbond request

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
        recipient,
        recipient_channel_id,
    )?;

    let res: Response = Response::new().add_event(unstake_request_event);

    Ok(res)
}

/// Process zkgm bond callback by calling process_bond
#[allow(clippy::too_many_arguments)]
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
    recipient: Option<String>,
    recipient_channel_id: Option<u32>,
) -> Result<Response, ContractError> {
    let status = STATUS.load(deps.storage)?;
    if status.bond_is_paused {
        return Err(ContractError::FunctionalityUnderMaintenance {});
    }
    let on_chain_recipient = validate_recipient(
        &deps.as_ref(),
        recipient.clone(),
        recipient_channel_id,
        Some(salt.clone()),
    )?;

    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom.clone();
    let sender = info.sender;
    let delegator = env.contract.address;

    let slippage_rate = match slippage {
        Some(rate) => rate,
        None => Decimal::from_ratio(Uint128::one(), Uint128::from(100_u32)),
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
        recipient.clone(),
        recipient_channel_id,
        on_chain_recipient,
    )?;

    if bond_data.mint_amount == Uint128::zero() {
        return Err(ContractError::InvalidMintAmount {});
    }

    // create bond event here
    let bond_event = BondEvent(
        sender.to_string(),
        sender.to_string(),
        amount,
        bond_data.delegated_amount,
        bond_data.mint_amount,
        bond_data.total_bond_amount,
        bond_data.total_supply,
        bond_data.exchange_rate,
        "".to_string(),
        env.block.time,
        coin_denom.clone(),
        recipient.clone(),
        recipient_channel_id,
        bond_data.reward_balance,
        bond_data.unclaimed_reward,
        None,
    );

    check_slippage(bond_data.mint_amount, expected, slippage_rate)?;

    deps.api
        .debug(&format!("{}: {:?}", env.block.time, bond_event));

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
        &deps.as_ref(),
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

    let Some(next_batch_action_time) = batch.next_batch_action_time else {
        return Err(ContractError::InvalidBatch {});
    };

    if env.block.time.seconds() < next_batch_action_time {
        return Err(ContractError::BatchNotReady {
            actual: env.block.time.seconds(),
            expected: next_batch_action_time,
        });
    }

    let Some(expected_native_unstaked) = batch.expected_native_unstaked else {
        return Err(ContractError::InvalidBatch {});
    };

    if amount > expected_native_unstaked {
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
            .amount,
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
                    .with_payload(payload_bin);
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
#[allow(clippy::too_many_arguments)]
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
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if let Some(fee_rate) = fee_rate {
        if fee_rate > Decimal::one() {
            return Err(ContractError::InvalidFeeRate { rate: fee_rate });
        }
    }

    let mut params = PARAMETERS.load(deps.storage)?;

    params.underlying_coin_denom = underlying_coin_denom
        .clone()
        .unwrap_or(params.underlying_coin_denom);
    params.liquidstaking_denom = liquidstaking_denom
        .clone()
        .unwrap_or(params.liquidstaking_denom);
    params.ucs03_relay_contract = ucs03_relay_contract
        .clone()
        .unwrap_or(params.ucs03_relay_contract);
    params.unbonding_time = unbonding_time.unwrap_or(params.unbonding_time);
    params.cw20_address = cw20_address.clone().unwrap_or(params.cw20_address);
    params.reward_address = reward_address.clone().unwrap_or(params.reward_address);

    params.fee_receiver = fee_receiver.clone().unwrap_or(params.fee_receiver);
    params.fee_rate = fee_rate.unwrap_or(params.fee_rate);
    params.min_bond = min_bond.unwrap_or(params.min_bond);
    params.min_unbond = min_unbond.unwrap_or(params.min_unbond);
    params.batch_limit = batch_limit.unwrap_or(params.batch_limit);
    params.transfer_handler = transfer_handler.clone().unwrap_or(params.transfer_handler);
    params.transfer_fee = transfer_fee.unwrap_or(params.transfer_fee);
    params.zkgm_token_minter = zkgm_token_minter
        .clone()
        .unwrap_or(params.zkgm_token_minter);
    params.underlying_coin_denom_symbol = underlying_coin_denom_symbol
        .clone()
        .unwrap_or(params.underlying_coin_denom_symbol);

    params.liquidstaking_denom_symbol = liquidstaking_denom_symbol
        .clone()
        .unwrap_or(params.liquidstaking_denom_symbol);

    if let Some(batch_period) = batch_period {
        params.batch_period = batch_period;
    };

    let cw20_addr_string = match cw20_address {
        Some(cw20) => cw20.to_string(),
        None => "".to_string(),
    };

    let mut reward_address_str = String::new();

    let mut msgs: Vec<CosmosMsg> = vec![];

    if let Some(reward_address) = reward_address {
        let msg: CosmosMsg = CosmosMsg::Distribution(DistributionMsg::SetWithdrawAddress {
            address: reward_address.to_string(),
        });
        msgs.push(msg);
        reward_address_str = reward_address.to_string();
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
            liquidstaking_denom.unwrap_or_default(),
        )
        .add_attribute(
            "underlying_coin_denom",
            underlying_coin_denom.unwrap_or_default(),
        )
        .add_attribute(
            "ucs03_relay_contract",
            ucs03_relay_contract.unwrap_or_default(),
        )
        .add_attribute("cw20_address", cw20_addr_string)
        .add_attribute("reward_address", reward_address_str)
        .add_attribute("transfer_fee", transfer_fee_str)
        .add_attribute("transfer_handler", transfer_handler.unwrap_or_default())
        .add_attribute("zkgm_token_minter", zkgm_token_minter.unwrap_or_default())
        .add_attribute(
            "underlying_coin_denom_symbol",
            underlying_coin_denom_symbol.unwrap_or_default(),
        )
        .add_attribute(
            "liquidstaking_denom_symbol",
            liquidstaking_denom_symbol.unwrap_or_default(),
        );
    Ok(res)
}

#[derive(Debug)]
pub struct StakerUndelegation {
    pub unstake_amount: Uint128,
    pub channel_id: Option<u32>,
    pub unstake_return_native_amount: Option<Uint128>,
    pub recipient: Option<String>,
    pub recipient_channel_id: Option<u32>,
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

    let Some(total_received_amount) = batch.received_native_unstaked else {
        return Err(ContractError::BatchIncompleteUnbonding {});
    };

    let mut unbonding_records =
        query_unreleased_unbond_record_from_batch(deps.storage, batch.id, params.batch_limit);

    let is_last_query = unbonding_records.len() < params.batch_limit as usize;

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
    for (i, ((staker, _), undelegation)) in staker_undelegation.iter().enumerate() {
        // if recipient channel id is set or channel id is set, it means that the receiver/recipient is on other chain
        // then if channel_id is set but without recipient channel id also without recipient, it will send back to staker via original channel id
        let is_on_chain_recipient = utils::validation::is_on_chain_recipient(
            &deps.as_ref(),
            undelegation.recipient.clone(),
            undelegation.recipient_channel_id,
        );

        let Some(unstake_return_native_amount) = undelegation.unstake_return_native_amount else {
            continue;
        };

        // if recipient channel id is set, it means that the receiver/recipient is on other chain
        // but if channel_id is set but recipient also recipient_channel_id is none, it will send to staker
        if !is_on_chain_recipient
            && (undelegation.channel_id.is_some() || undelegation.recipient_channel_id.is_some())
        {
            // if staker is from other chain
            // need to send transfer to transfer handler first before transfer to other chain via ucs03 contract
            let bank_msg = BankMsg::Send {
                to_address: transfer_handler.clone(),
                amount: vec![Coin {
                    denom: denom.clone(),
                    amount: unstake_return_native_amount,
                }],
            };
            let msg: CosmosMsg = CosmosMsg::Bank(bank_msg);
            send_msgs.push(msg);

            let target_channel_id = undelegation
                .recipient_channel_id
                .or(undelegation.channel_id)
                .ok_or(ContractError::InvalidChannelId {
                    msg: "out chain recipient must have channel id".to_string(),
                })?;

            let receiver = match undelegation.recipient.clone() {
                Some(rec) => rec,
                None => staker.clone(),
            };
            //after send bank msg to transfer handler, then call ucs03 on behalf of transfer handler to send token back
            let Some(salt) = salt.get(i).cloned() else {
                return Err(ContractError::InvalidSalt {});
            };
            let msg = get_unbonding_ucs03_transfer_cosmos_msg(
                deps.storage,
                lst_contract.clone(),
                receiver,
                target_channel_id,
                time,
                ucs03_relay_contract.clone(),
                unstake_return_native_amount,
                params.transfer_fee,
                denom.clone(),
                salt,
            )?;
            send_msgs.push(msg);
        } else {
            let recipient = match undelegation.recipient.clone() {
                Some(addr) => addr,
                None => staker.clone(),
            };
            let bank_msg = BankMsg::Send {
                to_address: recipient,
                amount: vec![Coin {
                    denom: denom.clone(),
                    amount: unstake_return_native_amount,
                }],
            };
            let msg: CosmosMsg = CosmosMsg::Bank(bank_msg);
            send_msgs.push(msg);
        }

        let ev = ProcessUnbondingEvent(
            id,
            undelegation.channel_id,
            staker.to_string(),
            unstake_return_native_amount,
            denom.clone(),
            env.block.time,
            undelegation.recipient.clone(),
            undelegation.recipient_channel_id,
        );
        events.push(ev);
    }

    if total_released_amount > Uint128::zero() {
        let ev = ProcessBatchUnbondingEvent(
            id,
            time,
            total_released_amount,
            total_received_amount,
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
        env.block.time, info.sender, channel_id, sender, payload
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
            recipient,
            recipient_channel_id,
        } => zkgm_bond(
            deps,
            env,
            info,
            channel_id,
            format!("{}", sender),
            amount,
            salt,
            slippage,
            expected,
            recipient,
            recipient_channel_id,
        ),
        ZkgmMessage::Unbond {
            amount,
            recipient,
            recipient_channel_id,
        } => zkgm_unbond(
            deps,
            env,
            info,
            channel_id,
            format!("{}", sender),
            amount,
            recipient,
            recipient_channel_id,
        ),
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

    if validators.is_empty() {
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
