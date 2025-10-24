use cosmwasm_std::{
    Attribute, BankMsg, Coin, CosmosMsg, DepsMut, Env, Reply, Response, SubMsg, Uint128, WasmMsg,
    attr, entry_point, from_json, to_json_binary, wasm_execute,
};

use crate::{
    error::ContractError,
    msg::{BondRewardsPayload, ExecuteRewardMsg, MintTokensPayload, ZkgmTransfer},
    state::{
        PARAMETERS, Parameters, QUOTE_TOKEN, REWARD_BALANCE, SPLIT_REWARD_QUEUE, SUPPLY_QUEUE,
        WITHDRAW_REWARD_QUEUE, WithdrawReward, WithdrawRewardQueue,
    },
    utils::calc::get_next_epoch,
    zkgm::protocol::{TokenPair, Ucs03Zkgm},
};
pub const MINT_CW20_TOKENS_REPLY_ID: u64 = 124;
pub const PROCESS_WITHDRAW_REWARD_REPLY_ID: u64 = 125;
pub const SPLIT_REWARD_REPLY_ID: u64 = 126;
pub const MINT_AND_SEND_ZKGM_REPLY_ID: u64 = 127;

#[entry_point]
/// Errors:
/// - Propagates errors from submessage replies and handlers.
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if !msg.result.is_ok() {
        let err = msg.result.unwrap_err();
        return Err(ContractError::ReplyError {
            message: err.clone(),
        });
    }

    match msg.id {
        MINT_AND_SEND_ZKGM_REPLY_ID => on_mint_and_send_zkgm(&deps, &env, msg),
        MINT_CW20_TOKENS_REPLY_ID => on_mint_cw20_tokens(deps, &env, msg),
        PROCESS_WITHDRAW_REWARD_REPLY_ID => on_process_rewards(deps, env, msg),
        SPLIT_REWARD_REPLY_ID => on_split_reward(deps, &env, msg),
        _ => Ok(Response::new()),
    }
}

#[allow(clippy::too_many_lines)]
fn on_mint_cw20_tokens(deps: DepsMut, env: &Env, msg: Reply) -> Result<Response, ContractError> {
    let params: Parameters = PARAMETERS.load(deps.storage)?;
    let payload: MintTokensPayload = from_json(msg.payload)?;

    // if recipient channel id is none, need to make sure recipient address is valid address on the chain where the contract is running
    let is_on_chain_recipient = crate::utils::validation::is_on_chain_recipient(
        &deps.as_ref(),
        &payload.recipient.clone(),
        payload.recipient_channel_id,
        &None,
    );

    // check to query balance of transfer handler or this contract
    // transfer handler is used to transfer cw20 minted token to other chain
    let msg = if !is_on_chain_recipient
        && (payload.channel_id.is_some() || payload.recipient_channel_id.is_some())
    {
        cw20::Cw20QueryMsg::Balance {
            address: params.transfer_handler.clone(),
        }
    } else {
        cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        }
    };

    let balance: cw20::BalanceResponse = deps
        .querier
        .query_wasm_smart(params.cw20_address.clone(), &msg)?;

    if balance.balance < payload.amount {
        return Err(ContractError::NotEnoughAvailableFund {});
    }

    let mut msgs: Vec<CosmosMsg> = vec![];

    // if staker is from other chain, we need to use transfer handler address to do transfer back to user
    // so we need to get correct authz execute msg to cw20 and ucs03 to handle the transfer
    // also need to attach required funds
    let mut quote_token_string = String::new();
    let amount = payload.amount;
    // check if payload has transfer fee, use it, otherwise use default transfer fee from parameters
    let transfer_fee = match payload.transfer_fee {
        Some(fee) => fee,
        None => params.transfer_fee,
    };

    // if recipient channel id is set or channel id is set, it means that the receiver/recipient is on other chain
    // then if channel_id is set but without recipient channel id also without recipient, it will send back to staker via original channel id
    if !is_on_chain_recipient
        && (payload.channel_id.is_some() || payload.recipient_channel_id.is_some())
    {
        let channel_id = match payload.recipient_channel_id {
            Some(channel_id) => channel_id,
            None => payload.channel_id.unwrap(),
        };

        let recipient = match payload.recipient.clone() {
            Some(rec) => rec,
            None => payload.staker.clone(),
        };
        let params = PARAMETERS.load(deps.storage)?;

        // create allowance msg to zkgm token minter
        msgs.push(
            wasm_execute(
                params.cw20_address.clone(),
                &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                    spender: params.zkgm_token_minter,
                    amount,
                    expires: None,
                },
                vec![],
            )?
            .into(),
        );

        let quote_token = QUOTE_TOKEN.load(deps.storage, channel_id)?;
        quote_token_string.clone_from(&quote_token.lst_quote_token);

        // create send ucs03 zkgm msg
        msgs.push(
            Ucs03Zkgm::new(
                params.ucs03_relay_contract.clone(),
                TokenPair {
                    base_token: params.cw20_address.to_string(),
                    quote_token: quote_token_string.clone(),
                },
            )
            .transfer_escrow_with_funds(
                &ZkgmTransfer {
                    sender: env.contract.address.to_string(),
                    amount,
                    recipient,
                    recipient_channel_id: channel_id,
                    salt: payload.salt,
                    time: env.block.time,
                },
                &[], // to transfer cw20 based token (liquid staking token), no need to attach funds
            )?,
        );
    } else {
        let receiver = match payload.recipient.clone() {
            Some(receiver) => receiver,
            None => payload.staker.clone(),
        };

        // if staker from same chain, this contract will send the cw20 staking token
        let msg = send_cw20(deps, amount, params.cw20_address.to_string(), receiver)?;
        msgs.push(msg);
    }

    let res: Response = Response::new()
        .add_messages(msgs)
        .add_attribute("action", "mint_cw20")
        .add_attribute("sender", payload.sender.clone())
        .add_attribute("staker", payload.staker.clone())
        .add_attribute("recipient", format!("{:?}", payload.recipient))
        .add_attribute("channel_id", payload.channel_id.unwrap_or(0).to_string())
        .add_attribute("amount", amount.to_string())
        .add_attribute("denom", params.liquidstaking_denom)
        .add_attribute("base_denom", params.cw20_address)
        .add_attribute("quote_token", quote_token_string)
        .add_attribute("transfer_handler", params.transfer_handler)
        .add_attribute("transfer_fee", transfer_fee)
        .add_attribute("staked_token_balance", balance.balance.to_string());

    Ok(res)
}

fn on_mint_and_send_zkgm(
    deps: &DepsMut,
    _env: &Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    let zkgm_transfer: ZkgmTransfer = from_json(msg.payload)?;
    let params = PARAMETERS.load(deps.storage)?;
    let quote_token = QUOTE_TOKEN.load(deps.storage, zkgm_transfer.recipient_channel_id)?;

    // construct send allowance and ucs03 zkgm send message
    let msgs: Vec<CosmosMsg> = vec![
        // 1. send allowance msg
        wasm_execute(
            params.cw20_address.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: params.zkgm_token_minter,
                amount: zkgm_transfer.amount,
                expires: None,
            },
            vec![],
        )?
        .into(),
        // 2. send to ucs03 msg without funds as we transfer cw20 denom
        Ucs03Zkgm::new(
            params.ucs03_relay_contract.clone(),
            TokenPair {
                base_token: params.cw20_address.to_string(),
                quote_token: quote_token.lst_quote_token.clone(),
            },
        )
        .transfer_escrow_with_funds(&zkgm_transfer, &[])?,
    ];

    let res: Response = Response::new()
        .add_messages(msgs)
        .add_attribute("action", "send_zkgm")
        .add_attribute("sender", zkgm_transfer.sender.clone())
        .add_attribute("recipient", zkgm_transfer.recipient)
        .add_attribute("channel_id", zkgm_transfer.recipient_channel_id.to_string())
        .add_attribute("amount", zkgm_transfer.amount.to_string())
        .add_attribute("denom", params.liquidstaking_denom)
        .add_attribute("base_denom", params.cw20_address)
        .add_attribute("quote_token", quote_token.lst_quote_token);

    Ok(res)
}

/// Call split reward to reward contract after withdraw reward
#[allow(clippy::needless_pass_by_value)]
fn on_process_rewards(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let payload: BondRewardsPayload = from_json(msg.payload)?;

    let mut msgs: Vec<SubMsg> = vec![];
    let mut attrs: Vec<Attribute> = vec![];

    // increment the reward balance on this contract as result of withdraw reward
    let mut reward_balance = REWARD_BALANCE.load(deps.storage)?;
    reward_balance += payload.amount;
    REWARD_BALANCE.save(deps.storage, &reward_balance)?;

    let mut split_reward_queue = SPLIT_REWARD_QUEUE.load(deps.storage)?;
    split_reward_queue.withdrawed_amount += payload.amount;
    SPLIT_REWARD_QUEUE.save(deps.storage, &split_reward_queue)?;

    if split_reward_queue.withdrawed_amount == split_reward_queue.target_amount {
        // if total amount that should be withdrawed is reached, we need to split reward
        let msg = ExecuteRewardMsg::SplitReward {};
        let msg_bin = to_json_binary(&msg)?;
        let split_reward_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: params.reward_address.to_string(),
            msg: msg_bin,
            funds: vec![],
        });

        attrs = vec![
            attr("action", "execute_split_reward"),
            attr("reward_contract", params.reward_address.clone().to_string()),
        ];

        // reset SPLIT_REWARD_QUEUE as we already call the split reward and total amount that should be withdrawed is reached
        SPLIT_REWARD_QUEUE.save(
            deps.storage,
            &WithdrawReward {
                target_amount: Uint128::zero(),
                withdrawed_amount: Uint128::zero(),
            },
        )?;

        let sub_msg: SubMsg = SubMsg::reply_always(split_reward_msg, SPLIT_REWARD_REPLY_ID);
        msgs.push(sub_msg);
    }

    let res: Response = Response::new().add_submessages(msgs).add_attributes(attrs);

    Ok(res)
}

/// Handle split reward call to reward contract reply
#[allow(clippy::needless_pass_by_value)]
fn on_split_reward(deps: DepsMut, env: &Env, _msg: Reply) -> Result<Response, ContractError> {
    // reset reward balance after split reward call success
    REWARD_BALANCE.save(deps.storage, &Uint128::new(0))?;
    let supply = SUPPLY_QUEUE.load(deps.storage)?;
    let block_height = env.block.height;

    let next_epoch = get_next_epoch(block_height, supply.epoch_period);
    let epoch_diff = next_epoch - block_height;

    // Only add one withdraw reward queue entry if epoch diff > 3 to trigger normalize reward
    if epoch_diff > 0 && epoch_diff != u64::from(supply.epoch_period) {
        let reward_queue = WithdrawRewardQueue {
            amount: Uint128::zero(),
            block: block_height,
        };
        WITHDRAW_REWARD_QUEUE.save(deps.storage, &vec![reward_queue])?;
    } else {
        // reset withdraw reward queue if epoch diff is 0 or equal to epoch period
        WITHDRAW_REWARD_QUEUE.save(deps.storage, &vec![])?;
    }

    let res: Response = Response::new().add_attribute("action", "on_split_reward");

    Ok(res)
}

/// Send or transfer token on same chain
/// Errors:
/// - Returns StdError on message serialization failure.
pub fn send(_deps: DepsMut, amount: Coin, receiver: &str) -> Result<Response, ContractError> {
    let bank_msg: BankMsg = BankMsg::Send {
        to_address: receiver.to_string(),
        amount: vec![amount.clone()],
    };

    let msg: CosmosMsg = CosmosMsg::Bank(bank_msg);

    let res: Response = Response::new()
        .add_message(msg)
        .add_attribute("action", "send")
        .add_attribute("receiver", receiver.to_string())
        .add_attribute("amount", amount.amount.to_string())
        .add_attribute("denom", amount.denom);
    Ok(res)
}

/// Send or transfer cw20 token on same chain
/// Errors:
/// - Returns StdError if CW20 message serialization fails.
pub fn send_cw20(
    _deps: DepsMut,
    amount: Uint128,
    cw20_address: String,
    recipient: String,
) -> Result<CosmosMsg, ContractError> {
    let cw20_execute_transfer_msg = cw20::Cw20ExecuteMsg::Transfer { recipient, amount };
    let msg_bin = to_json_binary(&cw20_execute_transfer_msg).unwrap();
    let cw20_transfer_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw20_address,
        msg: msg_bin,
        funds: vec![],
    });
    Ok(cw20_transfer_msg)
}
