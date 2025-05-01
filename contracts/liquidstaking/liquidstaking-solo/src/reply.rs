use std::str::FromStr;

use crate::error::ContractError;
use crate::msg::{BondRewardsPayload, ExecuteRewardMsg, MintTokensPayload};
use crate::state::{
    Parameters, WithdrawReward, PARAMETERS, QUOTE_TOKEN, REWARD_BALANCE, SPLIT_REWARD_QUEUE,
};
use cosmwasm_std::{
    attr, entry_point, from_json, to_json_binary, Attribute, BankMsg, Coin, CosmosMsg, DepsMut,
    Env, Reply, Response, SubMsg, Uint128, WasmMsg,
};
use unionlabs_primitives::Bytes;

pub const MINT_CW20_TOKENS_REPLY_ID: u64 = 124;
pub const PROCESS_WITHDRAW_REWARD_REPLY_ID: u64 = 125;
pub const SPLIT_REWARD_REPLY_ID: u64 = 126;
pub const TRANSFER_STAKING_TOKEN_ID: u64 = 127;

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if !msg.result.is_ok() {
        let err = msg.result.unwrap_err();
        return Err(ContractError::ReplyError {
            message: err.to_string(),
        });
    }

    match msg.id {
        MINT_CW20_TOKENS_REPLY_ID => on_mint_cw20_tokens(deps, env, msg),
        PROCESS_WITHDRAW_REWARD_REPLY_ID => on_process_rewards(deps, env, msg),
        SPLIT_REWARD_REPLY_ID => on_split_reward(deps, env, msg),
        _ => Ok(Response::new()),
    }
}

fn on_mint_cw20_tokens(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    let params: Parameters = PARAMETERS.load(deps.storage)?;
    let payload: MintTokensPayload = from_json(msg.payload)?;

    let msg = match payload.channel_id {
        Some(_) => cw20::Cw20QueryMsg::Balance {
            address: params.transfer_handler.to_string(),
        },
        None => cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
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
    if payload.staker != payload.sender && payload.channel_id.is_some() {
        let channel_id = payload.channel_id.unwrap();
        let params = PARAMETERS.load(deps.storage)?;

        let mut msgs = vec![];

        // allow/approve ucs03 to transfer on behalf of transfer handler via authz
        let allowance_msg = crate::utils::authz::get_authz_increase_allowance_msg(
            params.transfer_handler.clone(),
            env.contract.address.to_string(),
            params.cw20_address.to_string(),
            params.zkgm_token_minter,
            payload.amount,
            vec![],
        )?;

        msgs.push(allowance_msg);

        let mut funds = vec![];
        if params.transfer_fee > Uint128::zero() {
            funds.push(Coin {
                amount: params.transfer_fee, // need to add transfer fee
                denom: params.underlying_coin_denom,
            });
        }

        let quote_token = QUOTE_TOKEN.load(deps.storage, channel_id)?;

        let authz_ucs03_msg = crate::utils::authz::get_authz_ucs03_transfer(
            params.cw20_address.to_string(),
            params.transfer_handler,
            env.contract.address.to_string(),
            env.block.time,
            params.ucs03_relay_contract.clone(),
            channel_id,
            Bytes::from_str(payload.staker.as_str()).unwrap(),
            params.cw20_address.to_string(),
            payload.amount.clone(),
            Bytes::from_str(quote_token.lst_quote_token.as_str()).unwrap(),
            payload.amount,
            funds.clone(),
            unionlabs_primitives::H256::from_str(payload.salt.as_str()).unwrap(),
        )?;

        msgs.push(authz_ucs03_msg);
    } else {
        // if staker from same chain, this contract will send the cw20 staking token
        let msg = send_cw20(
            deps,
            payload.amount,
            params.cw20_address.to_string(),
            payload.staker.clone(),
        )?;
        msgs.push(msg);
    }

    let res: Response = Response::new()
        .add_messages(msgs)
        .add_attribute("action", "mint_cw20")
        .add_attribute("receiver", payload.staker.to_string())
        .add_attribute("amount", payload.amount.to_string())
        .add_attribute("denom", params.liquidstaking_denom)
        .add_attribute("base_denom", params.cw20_address)
        .add_attribute("staked_token_balance", balance.balance.to_string());
    Ok(res)
}

/// Call split reward to reward contract after withdraw reward
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
fn on_split_reward(deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    // reset reward balance after split reward call success
    REWARD_BALANCE.save(deps.storage, &Uint128::new(0))?;

    let res: Response = Response::new().add_attribute("action", "on_split_reward");

    Ok(res)
}

/// Send or transfer token on same chain
pub fn send(_deps: DepsMut, amount: Coin, receiver: String) -> Result<Response, ContractError> {
    let bank_msg: BankMsg = BankMsg::Send {
        to_address: receiver.clone(),
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
