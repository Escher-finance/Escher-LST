use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{
    Addr, Attribute, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, DistributionMsg, Env, Event,
    MessageInfo, Response, StdError, SubMsg, Uint128, WasmMsg, attr, from_json, to_json_binary,
    wasm_execute,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use unionlabs_primitives::Bytes;

use crate::{
    error::ContractError,
    event::{
        BatchReceivedEvent, BatchReleasedEvent, BondEvent, ProcessBatchUnbondingEvent,
        ProcessRewardsEvent, ProcessUnbondingEvent, SplitRewardEvent, UpdateValidatorsEvent,
    },
    helpers,
    msg::{
        BatchReceivedAmount, BondRewardsPayload, Cw20PayloadMsg, ExecuteMsg, ExecuteRewardMsg,
        Recipient, RewardMigrateMsg, ZkgmTransfer,
    },
    query::query_unreleased_unbond_record_from_batch,
    reply::PROCESS_WITHDRAW_REWARD_REPLY_ID,
    state::{
        Chain, PARAMETERS, PENDING_BATCH_ID, QUOTE_TOKEN, QuoteToken, REWARD_BALANCE,
        SPLIT_REWARD_QUEUE, STATE, STATUS, SUPPLY_QUEUE, Status, VALIDATORS_REGISTRY, Validator,
        WITHDRAW_REWARD_QUEUE, WithdrawReward, WithdrawRewardQueue,
    },
    types::ChannelId,
    utils::{
        self,
        batch::{BatchStatus, batches},
        calc::{
            calculate_exchange_rate, calculate_fee_from_reward, get_last_epoch_block,
            get_next_epoch, normalize_withdraw_reward_queue, to_uint128,
        },
        delegation::{get_actual_total_delegated, get_actual_total_reward, submit_pending_batch},
        transfer::{self, get_send_bank_msg, ibc_transfer_msg},
        validation::{
            split_and_validate_recipient, validate_recipient, validate_remote_sender,
            validate_required_coin, validate_validators,
        },
    },
    zkgm::protocol::ucs03_transfer_v2,
};

/// process bond call to contract
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
pub fn bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    slippage: Option<Decimal>,
    expected: Uint128,
    recipient: Recipient,
) -> Result<Response, ContractError> {
    let status = STATUS.load(deps.storage)?;
    if status.bond_is_paused {
        return Err(ContractError::FunctionalityUnderMaintenance {});
    }
    let params = PARAMETERS.load(deps.storage)?;

    let coin = validate_required_coin(
        &info.funds,
        &Coin {
            amount: params.min_bond,
            denom: params.underlying_coin_denom.clone(),
        },
    )?;

    // handle delegation to validators
    let (mut msgs, bond_data) = utils::delegation::delegate(
        deps.storage,
        deps.querier,
        env.clone(),
        coin.amount,
        expected,
        slippage,
    )?;

    let (the_recipient, recipient_channel_id, recipient_ibc_channel_id) =
        split_and_validate_recipient(deps.storage, recipient.clone())?;

    match recipient {
        Recipient::OnChain { address } => {
            // mint staked token to on chain recipient address
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: bond_data.cw20_address,
                msg: to_json_binary(&cw20::Cw20ExecuteMsg::Mint {
                    recipient: address.to_string(),
                    amount: bond_data.mint_amount,
                })?,
                funds: vec![],
            }));
        }
        Recipient::Zkgm {
            address: _,
            channel_id: _,
        } => {
            // mint staked token to sender because it will transfer via ucs03 zkgm from original sender
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: bond_data.cw20_address,
                msg: to_json_binary(&cw20::Cw20ExecuteMsg::Mint {
                    recipient: info.sender.to_string(),
                    amount: bond_data.mint_amount,
                })?,
                funds: vec![],
            }));
        }
        Recipient::IBC {
            address: _,
            ibc_channel_id: _,
        } => return Err(ContractError::FunctionalityUnderMaintenance {}),
    }

    // create bond event here
    let bond_event = BondEvent(
        info.sender.to_string(),
        info.sender.to_string(),
        coin.amount,
        bond_data.delegated_amount,
        bond_data.mint_amount,
        bond_data.total_bond_amount,
        bond_data.total_supply,
        bond_data.exchange_rate,
        String::new(),
        env.block.time,
        params.underlying_coin_denom.clone(),
        the_recipient.clone(),
        recipient_channel_id,
        bond_data.reward_balance,
        bond_data.unclaimed_reward,
        recipient_ibc_channel_id,
    );

    let res: Response = Response::new()
        .add_messages(msgs)
        .add_event(bond_event)
        .add_attributes(vec![
            attr("action", "bond"),
            attr("from", info.sender.clone()),
            attr("staker", info.sender.clone()),
            attr("recipient", the_recipient.unwrap_or(info.sender.into())),
            attr("channel_id", recipient_channel_id.unwrap_or(0).to_string()),
            attr("bond_amount", coin.amount.to_string()),
            attr("denom", params.underlying_coin_denom.clone()),
            attr("minted", bond_data.mint_amount),
            attr("exchange_rate", bond_data.exchange_rate.to_string()),
        ]);

    Ok(res)
}

/// Process remote bond call to contract, this will not send to any other chain address as send staked token back is attached on zkgm Calls
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
pub fn remote_bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    min_mint_amount: Uint128,
    mint_to_address: Addr,
) -> Result<Response, ContractError> {
    let status = STATUS.load(deps.storage)?;
    if status.bond_is_paused {
        return Err(ContractError::FunctionalityUnderMaintenance {});
    }

    let params = PARAMETERS.load(deps.storage)?;

    let coin = validate_required_coin(
        &info.funds,
        &Coin {
            amount: params.min_bond,
            denom: params.underlying_coin_denom.clone(),
        },
    )?;

    // assume sender is cw-account contract address if contract creator is the ucs03 contract
    validate_remote_sender(deps.querier, &info.sender, &params)?;

    // handle delegation to validators
    let (mut msgs, bond_data) = utils::delegation::delegate(
        deps.storage,
        deps.querier,
        env.clone(),
        coin.amount,
        min_mint_amount,
        None,
    )?;

    // mint staked token to mint_to_address
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: bond_data.cw20_address,
        msg: to_json_binary(&cw20::Cw20ExecuteMsg::Mint {
            recipient: mint_to_address.to_string(),
            amount: bond_data.mint_amount,
        })?,
        funds: vec![],
    }));

    // create bond event here
    let bond_event = BondEvent(
        info.sender.to_string(),
        info.sender.to_string(),
        bond_data.bond_amount,
        bond_data.delegated_amount,
        bond_data.mint_amount,
        bond_data.total_bond_amount,
        bond_data.total_supply,
        bond_data.exchange_rate,
        String::new(),
        env.block.time,
        bond_data.denom.clone(),
        None,
        None,
        bond_data.reward_balance,
        bond_data.unclaimed_reward,
        None,
    );

    let res: Response = Response::new()
        .add_messages(msgs)
        .add_event(bond_event)
        .add_attributes(vec![attr("action", "remote_bond")]);

    Ok(res)
}

/// Process receive msg from liquid stoken cw20 contract with embedded unbond payload msg to do unbond/unstake
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
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
    // make sure the sender is the cw20 contract because only cw20 contract can call this function
    if info.sender != params.cw20_address {
        return Err(ContractError::Unauthorized {});
    }

    let state = STATE.load(deps.storage)?;
    if state.exchange_rate < Decimal::one() {
        return Err(ContractError::InvalidExchangeRate {});
    }

    let sender = cw20_msg.sender.clone();
    let the_staker: String = sender.clone();
    let delegator = env.contract.address.clone();

    let payload_msg: Cw20PayloadMsg = from_json(cw20_msg.msg)?;

    // make sure the payload is Unstake
    if !matches!(
        payload_msg,
        Cw20PayloadMsg::Unstake {
            recipient: _,
            recipient_channel_id: _,
            recipient_ibc_channel_id: _,
        }
    ) {
        return Err(ContractError::InvalidPayload {});
    }

    // get the recipient, recipient channel id and recipient_ibc_channel_id from payload_msg
    let (recipient, recipient_channel_id, recipient_ibc_channel_id) = match payload_msg {
        Cw20PayloadMsg::Unstake {
            recipient,
            recipient_channel_id,
            recipient_ibc_channel_id,
        } => (recipient, recipient_channel_id, recipient_ibc_channel_id),
    };

    validate_recipient(
        &deps,
        recipient.clone(),
        recipient_channel_id,
        recipient_ibc_channel_id.clone(),
        &Some(String::new()),
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
        &env.clone(),
        deps.storage,
        sender.clone(),
        the_staker.clone(),
        unbond_amount,
        None,
        recipient,
        recipient_channel_id,
        recipient_ibc_channel_id,
    )?;

    let res: Response = Response::new().add_event(unstake_request_event);

    Ok(res)
}

/// Process pending batch and execute it
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
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
        &validators_reg.clone(),
    )?;

    let res: Response = Response::new().add_messages(msgs).add_events(events);

    Ok(res)
}

// Set the batch received amount and set the batch status to received
// This will be called by backend and the amount data is pulled from indexer when batch complete unbonding is already executed on chain
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
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

    let next_action_time = batch
        .next_batch_action_time
        .ok_or(ContractError::BatchNextActionTimeNotSet)?;

    let expected_native_unstaked = batch
        .expected_native_unstaked
        .ok_or(ContractError::BatchExpectedNativeUnstakedNotSet)?;

    if env.block.time.seconds() < next_action_time {
        return Err(ContractError::BatchNotReady {
            actual: env.block.time.seconds(),
            expected: next_action_time,
        });
    }

    if amount > expected_native_unstaked || amount == Uint128::zero() {
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
/// /// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
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
        delegator.as_ref(),
        payment.amount,
        params.underlying_coin_denom.clone(),
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
        &coin_denom,
        &validators_list,
    )?;

    state.total_delegated_amount = delegated_amount;
    let total_reward = get_actual_total_reward(
        deps.storage,
        deps.querier,
        delegator.to_string(),
        &coin_denom,
        &validators_list,
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
        attr("denom", coin_denom.clone()),
        attr("exchange_rate", state.exchange_rate.to_string()),
    ]);

    Ok(res)
}

/// Process rewards by withdraw delegator reward then call redelegate to reward contract on reply
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
pub fn process_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom;
    let sender = info.sender;
    let delegator = env.contract.address;
    let mut sub_msgs: Vec<SubMsg> = vec![];

    let mut attrs = vec![attr("action", "process_rewards"), attr("from", sender)];

    let mut total_amount: Uint128 = Uint128::zero();

    let reward_balance_state = crate::state::REWARD_BALANCE.load(deps.storage)?;
    let reward_queue = WITHDRAW_REWARD_QUEUE.load(deps.storage)?;
    let supply_queue = SUPPLY_QUEUE.load(deps.storage)?;

    let (new_reward_balance, _) = normalize_withdraw_reward_queue(
        env.block.height,
        reward_balance_state,
        reward_queue,
        supply_queue.epoch_period,
    );

    crate::state::REWARD_BALANCE.save(deps.storage, &new_reward_balance)?;

    for validator in validators_reg.validators {
        let delegation_rewards = deps
            .querier
            .query_delegation_rewards(delegator.clone(), validator.address.clone())?;

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
                validator: validator.address.clone(),
            });

        if payload.amount != Uint128::zero() {
            let payload_bin = to_json_binary(&payload)?;

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

    let ev = ProcessRewardsEvent(total_amount, new_reward_balance);
    let res: Response = Response::new()
        .add_attributes(attrs)
        .add_event(ev)
        .add_submessages(sub_msgs);

    Ok(res)
}

/// Update the ownership of the contract.
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
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
    }

    cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;

    let res: Response = Response::new().add_attribute("action", "update_ownership");

    Ok(res)
}

/// Set contract parameters
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
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
    transfer_handler: Option<String>,
    transfer_fee: Option<Uint128>,
    zkgm_token_minter: Option<String>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if let Some(rate) = fee_rate {
        if rate > Decimal::one() {
            return Err(ContractError::InvalidFeeRate {});
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

    if let Some(batch_period) = batch_period {
        params.batch_period = batch_period;
    }

    // update epoch period in SUPPLY QUEUE
    if let Some(epoch_period) = epoch_period {
        let mut supply_queue = SUPPLY_QUEUE.load(deps.storage)?;
        supply_queue.epoch_period = epoch_period;
        SUPPLY_QUEUE.save(deps.storage, &supply_queue)?;
    }

    let cw20_addr_string = match cw20_address {
        Some(cw20) => cw20.to_string(),
        None => String::new(),
    };
    let mut reward_address_str = String::new();

    let mut msgs: Vec<CosmosMsg> = vec![];

    if let Some(reward_address) = reward_address {
        let msg: CosmosMsg = CosmosMsg::Distribution(DistributionMsg::SetWithdrawAddress {
            address: reward_address.clone().to_string(),
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
        .add_attribute("reward_address", reward_address_str);

    Ok(res)
}

#[derive(Debug)]
pub struct StakerUndelegation {
    pub unstake_amount: Uint128,
    pub channel_id: Option<u32>,
    pub unstake_return_native_amount: Option<Uint128>,
    pub recipient: Option<String>,
    pub recipient_channel_id: Option<u32>,
    pub recipient_ibc_channel_id: Option<String>,
}

/// Process received batch to release the native token back to user so user doesn't need to manually withdraw token
/// 1. Get all unbonding records from pending batch
/// 2. Get how much every user `unstaked_native_amount` result base on ratio of the user unstaked token to total liquid staked on current batch
/// 3. Set unbond records to released and set released height
/// 4. Generate cosmos msg to send token back to user
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::too_many_lines, clippy::needless_pass_by_value)]
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

    let Some(total_received_amount) = batch.received_native_unstaked else {
        return Err(ContractError::BatchIncompleteUnbonding {});
    };

    let mut unbonding_records =
        query_unreleased_unbond_record_from_batch(deps.storage, batch.id, params.batch_limit)?;

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

    let mut quote_token_map: HashMap<u32, QuoteToken> = HashMap::new();

    for (i, ((staker, _), undelegation)) in staker_undelegation.iter().enumerate() {
        // if recipient channel id is set or channel id is set, it means that the receiver/recipient is on other chain
        // then if channel_id is set but without recipient channel id also without recipient, it will send back to staker via original channel id
        let is_on_chain_recipient = utils::validation::is_on_chain_recipient(
            &deps.as_ref(),
            &undelegation.recipient.clone(),
            undelegation.recipient_channel_id,
            &undelegation.recipient_ibc_channel_id.clone(),
        );

        let salt = if let Some(salt) = salt.get(i) {
            salt.clone()
        } else {
            return Err(ContractError::NoSalt);
        };

        let Some(unstake_return_native_amount) = undelegation.unstake_return_native_amount else {
            return Err(ContractError::InvalidUnstakeReturnNativeAmount);
        };

        if is_on_chain_recipient {
            let msg = get_send_bank_msg(
                staker,
                undelegation.recipient.clone().as_ref(),
                &denom,
                unstake_return_native_amount,
            );
            send_msgs.push(msg);
        } else {
            // if recipient channel id is set, it means that the receiver/recipient is on other chain
            // but if channel_id is set but recipient also recipient_channel_id is none, it will send to staker
            if let Some(recipient_channel_id) = undelegation.recipient_channel_id {
                // get quote token for the channel id
                let quote_token = if let Some(qt) = quote_token_map.get(&recipient_channel_id) {
                    qt.clone()
                } else {
                    let qt = QUOTE_TOKEN.load(deps.storage, recipient_channel_id)?;
                    quote_token_map.insert(recipient_channel_id, qt.clone());
                    qt
                };

                let Some(recipient) = undelegation.recipient.as_ref() else {
                    return Err(ContractError::InvalidAddress {
                        kind: "recipient".to_string(),
                        address: "blank".to_string(),
                        reason: "recipient is required when recipient_channel_id is set"
                            .to_string(),
                    });
                };

                // send native token back via ucs03
                let msg = transfer::transfer_escrow_v2(
                    &ucs03_relay_contract,
                    &ZkgmTransfer {
                        sender: lst_contract.to_string(),
                        amount: unstake_return_native_amount,
                        recipient: recipient.clone(),
                        recipient_channel_id,
                        salt,
                    },
                    &denom,
                    &quote_token.quote_token,
                    time,
                )?;
                send_msgs.push(msg);
            } else if let Some(recipient_ibc_channel_id) = &undelegation.recipient_ibc_channel_id {
                if let Some(recipient) = &undelegation.recipient {
                    let msg = ibc_transfer_msg(
                        recipient_ibc_channel_id.clone(),
                        recipient.clone(),
                        unstake_return_native_amount,
                        &denom,
                        time,
                    );
                    send_msgs.push(msg);
                }
            }
        }

        let ev = ProcessUnbondingEvent(
            id,
            undelegation.channel_id,
            staker.clone(),
            unstake_return_native_amount,
            denom.clone(),
            env.block.time,
            undelegation.recipient.clone(),
            undelegation.recipient_channel_id,
            undelegation.recipient_ibc_channel_id.clone(),
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
            &unbond_record_ids,
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

/// Update the ownership of the contract.
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
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
    validators_reg.validators.clone_from(&validators);
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

/// Update the quote token of the contract for specific `channel_id`
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
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
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
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

/// Split reward to restake and send fee to fee receiver according to fee rate
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
pub fn split_reward(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = PARAMETERS.load(deps.storage)?;

    let lst_contract_address = env.contract.address;
    // only liquid staking contract able to call this function
    if info.sender != lst_contract_address {
        return Err(ContractError::Unauthorized {});
    }

    // first need to get this contract balance
    let balance = deps.querier.query_balance(
        lst_contract_address.clone(),
        config.underlying_coin_denom.clone(),
    )?;

    let mut msgs: Vec<CosmosMsg> = vec![];

    if balance.amount == Uint128::zero() {
        return Err(ContractError::NotEnoughFund {});
    }

    // check total balance from reward
    let reward_balance = REWARD_BALANCE.load(deps.storage)?;

    let mut balance_to_split = reward_balance;

    if balance.amount < balance_to_split {
        balance_to_split = balance.amount;
    }

    let mut attrs: Vec<Attribute> = vec![
        attr("action", "split_reward"),
        attr("fee_rate", format!("{:?}", config.fee_rate)),
        attr("amount", balance.amount.to_string()),
        attr("fee_receiver", config.fee_receiver.to_string()),
        attr("time", format!("{}", env.block.time.nanos())),
    ];
    let (redelegate, fee) = helpers::split_revenue(
        balance_to_split,
        config.fee_rate,
        &config.underlying_coin_denom,
    );

    // Send the fee to revenue receiver
    let bank_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: config.fee_receiver.to_string(),
        amount: vec![fee.clone()],
    });

    msgs.push(bank_msg);

    // Redelegate by call the LST Contract and attach the funds
    let lst: helpers::LstTemplateContract = helpers::LstTemplateContract(lst_contract_address);
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

/// Set status
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
pub fn set_status(
    deps: DepsMut,
    info: MessageInfo,
    status: Status,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    STATUS.save(deps.storage, &status)?;
    Ok(Response::new())
}

/// Set chain
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
pub fn set_chain(
    deps: DepsMut,
    info: MessageInfo,
    chain: Chain,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    crate::state::CHAINS.save(deps.storage, chain.ucs03_channel_id, &chain)?;
    Ok(Response::new())
}

/// Remove chain
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
pub fn remove_chain(
    deps: DepsMut,
    info: MessageInfo,
    channel_id: u32,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    crate::state::CHAINS.remove(deps.storage, channel_id);
    Ok(Response::new())
}

// Normalize reward only run when there is withdraw reward queue entry on active epoch period range to make sure the reward amount is normalized near end of epoch
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
pub fn normalize_reward(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let supply_queue = SUPPLY_QUEUE.load(deps.storage)?;

    let block_height = env.block.height;
    let mut next_epoch = get_next_epoch(block_height, supply_queue.epoch_period);

    let epoch_diff = next_epoch - block_height;

    let mut last_epoch = get_last_epoch_block(block_height, supply_queue.epoch_period);

    if epoch_diff.is_multiple_of(u64::from(supply_queue.epoch_period)) {
        last_epoch -= u64::from(supply_queue.epoch_period);
        next_epoch -= u64::from(supply_queue.epoch_period);
    }
    if epoch_diff > 5 && epoch_diff < u64::from(supply_queue.epoch_period) {
        return Err(ContractError::NoRewardToNormalize {
            msg: format!(
                "incorrect block height: current height: {block_height}, next epoch: {next_epoch}, only can normalize reward on end of epoch period range",
            ),
        });
    }

    let mut reward_queue = WITHDRAW_REWARD_QUEUE.load(deps.storage)?;
    if reward_queue.is_empty() {
        return Err(ContractError::NoRewardToNormalize {
            msg: "withdraw reward queue is empty".to_string(),
        });
    }

    // only normalize(add unclaimed reward to withdraw reward queue) if the existing queue is in the current epoch period
    for queue in &mut reward_queue {
        if queue.block < last_epoch {
            return Err(ContractError::NoRewardToNormalize {
                msg: "withdraw reward queue belongs to previous epoch".to_string(),
            });
        }
    }

    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    let delegator = env.contract.address;
    let unclaimed_reward = utils::delegation::get_unclaimed_reward(
        deps.querier,
        delegator.to_string(),
        &params.underlying_coin_denom,
        &validators_list,
    )?;
    let reward_balance = REWARD_BALANCE.load(deps.storage)?;

    if unclaimed_reward != Uint128::zero() {
        reward_queue.push(WithdrawRewardQueue {
            amount: unclaimed_reward,
            block: block_height,
        });
    }

    WITHDRAW_REWARD_QUEUE.save(deps.storage, &reward_queue)?;

    Ok(Response::new()
        .add_attribute("action", "normalize_reward")
        .add_attribute("unclaimed_reward", unclaimed_reward)
        .add_attribute("reward_balance", reward_balance)
        .add_attribute("current_height", block_height.to_string())
        .add_attribute("last_epoch", last_epoch.to_string())
        .add_attribute("next_epoch", next_epoch.to_string()))
}

/// Inject some amount of underlying coin denom to be staked without minting new cw20 token
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
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

    let (msgs, inject_data) = utils::delegation::inject(
        deps.storage,
        deps.querier,
        &contract_addr,
        amount,
        &params,
        env.block.height,
    )?;

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

/// Add new ibc channel config
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
pub fn add_ibc_channel(
    deps: DepsMut,
    info: MessageInfo,
    ibc_channel_id: String,
    prefix: String,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    crate::state::IBC_CHANNELS.save(deps.storage, ibc_channel_id, &prefix)?;
    Ok(Response::new())
}

/// Remove ibc channel config
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
pub fn remove_ibc_channel(
    deps: DepsMut,
    info: MessageInfo,
    ibc_channel_id: String,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    crate::state::IBC_CHANNELS.remove(deps.storage, ibc_channel_id);
    Ok(Response::new())
}

/// Process unbond request from user to unstake some amount of liquid staking token
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
pub fn remote_unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    recipient: Recipient,
) -> Result<Response, ContractError> {
    let status = STATUS.load(deps.storage)?;
    if status.unbond_is_paused {
        return Err(ContractError::FunctionalityUnderMaintenance {});
    }

    let params = PARAMETERS.load(deps.storage)?;
    // assume sender is cw-account contract address if contract creator is the ucs03 contract
    validate_remote_sender(deps.querier, &info.sender, &params)?;

    let state = STATE.load(deps.storage)?;
    if state.exchange_rate < Decimal::one() {
        return Err(ContractError::InvalidExchangeRate {});
    }

    let (recipient, recipient_channel_id, recipient_ibc_channel_id) =
        split_and_validate_recipient(deps.storage, recipient)?;

    let unstake_request_event = utils::delegation::unstake_request_in_batch(
        &env.clone(),
        deps.storage,
        info.sender.to_string(),
        info.sender.to_string(),
        amount,
        None,
        recipient,
        recipient_channel_id,
        recipient_ibc_channel_id,
    )?;

    let response = Response::new()
        .add_message(wasm_execute(
            params.cw20_address.to_string(),
            &Cw20ExecuteMsg::TransferFrom {
                owner: info.sender.to_string(),
                recipient: env.contract.address.to_string(),
                amount,
            },
            vec![],
        )?)
        .add_event(unstake_request_event);

    Ok(response)
}

/// Transfer liquid staking token to other chain via ucs03
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
pub fn transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    to: String,
    channel_id: u32,
    salt: String,
) -> Result<Response, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;

    let coin = info
        .funds
        .iter()
        .find(|x| x.denom == params.underlying_coin_denom && x.amount > Uint128::zero())
        .cloned()
        .ok_or_else(|| ContractError::NoAsset {})?;

    if coin.amount < amount {
        return Err(ContractError::NotEnoughFund {});
    }

    let Ok(recipient_address) = Bytes::from_str(to.as_str()) else {
        return Err(ContractError::InvalidAddress {
            kind: "recipient".into(),
            address: to,
            reason: "address must be in hex and starts with 0x".to_string(),
        });
    };

    let salt: unionlabs_primitives::H256 = match unionlabs_primitives::H256::from_str(salt.as_str())
    {
        Ok(s) => s,
        Err(e) => {
            return Err(ContractError::Std(StdError::generic_err(format!(
                "failed to parse salt: {salt}, reason: {e}"
            ))));
        }
    };

    let Some(channel_id) = ChannelId::from_raw(channel_id) else {
        return Err(ContractError::InvalidChannelId {});
    };

    let msg = ucs03_transfer_v2(
        deps,
        env,
        info.sender.as_ref(),
        recipient_address,
        amount,
        channel_id,
        salt,
    )?;

    let response = Response::new()
        .add_message(msg)
        .add_attribute("action", "transfer_lst")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("to", to.clone())
        .add_attribute("amount", amount.to_string())
        .add_attribute("channel_id", channel_id.to_string());

    Ok(response)
}

/// Slash batch by setting the new correct received amount for each batch id
/// # Result
/// Will return result of `cosmwasm_std::Response`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
pub fn slash_batch(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_received_amounts: Vec<BatchReceivedAmount>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut events: Vec<Event> = vec![];
    for BatchReceivedAmount {
        id: batch_id,
        received: received_amount,
    } in &new_received_amounts
    {
        let mut batch = batches().load(deps.storage, *batch_id)?;
        if batch.status != BatchStatus::Submitted {
            return Err(ContractError::BatchStatusIncorrect {
                actual: batch.status,
                expected: BatchStatus::Submitted,
            });
        }

        let next_action_time = batch
            .next_batch_action_time
            .ok_or(ContractError::BatchNextActionTimeNotSet)?;

        let expected_native_unstaked = batch
            .expected_native_unstaked
            .ok_or(ContractError::BatchExpectedNativeUnstakedNotSet)?;

        if env.block.time.seconds() < next_action_time {
            return Err(ContractError::BatchNotReady {
                actual: env.block.time.seconds(),
                expected: next_action_time,
            });
        }

        if *received_amount > expected_native_unstaked {
            return Err(ContractError::SlashBatchReceivedAmountExceedExpected {
                batch_id: *batch_id,
                received_amount: *received_amount,
                expected_native_unstaked,
            });
        }

        batch.received_native_unstaked = Some(*received_amount);
        batch.update_status(BatchStatus::Received, None);

        batches().save(deps.storage, *batch_id, &batch)?;

        events.push(BatchReceivedEvent(
            batch.id,
            received_amount.to_string(),
            env.block.time,
        ));
        events.push(
            Event::new("slash_batch")
                .add_attribute("batch_id", batch_id.to_string())
                .add_attribute("received_amount", received_amount.to_string()),
        );
    }

    let response = Response::new().add_events(events);

    Ok(response)
}
