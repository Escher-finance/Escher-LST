use std::str::FromStr;

use crate::error::ContractError;
use crate::msg::{BondRewardsPayload, ExecuteRewardMsg, MintTokensPayload};
use crate::state::{Parameters, PARAMETERS, QUOTE_TOKEN, REWARD_BALANCE};
use crate::utils;
use cosmwasm_std::{
    attr, entry_point, from_json, to_json_binary, Attribute, BankMsg, Coin, CosmosMsg, DepsMut,
    Env, Reply, Response, Uint128, Uint256, WasmMsg,
};
use unionlabs_primitives::{Bytes, H256};

pub const MINT_CW20_TOKENS_REPLY_ID: u64 = 124;
pub const PROCESS_WITHDRAW_REWARD_REPLY_ID: u64 = 125;

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
        _ => Ok(Response::new()),
    }
}

fn on_mint_cw20_tokens(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    let params: Parameters = PARAMETERS.load(deps.storage)?;
    let payload: MintTokensPayload = from_json(msg.payload)?;

    let msg = cw20::Cw20QueryMsg::Balance {
        address: env.contract.address.to_string(),
    };

    let balance: cw20::BalanceResponse = deps
        .querier
        .query_wasm_smart(params.cw20_address.clone(), &msg)?;

    if balance.balance < payload.amount {
        return Err(ContractError::NotEnoughAvailableFund {});
    }

    let mut msgs: Vec<CosmosMsg> = vec![];

    if payload.staker != payload.sender && payload.channel_id.is_some() {
        let allowance_msg = cw20::Cw20ExecuteMsg::IncreaseAllowance {
            spender: params.ucs03_relay_contract.clone(),
            amount: payload.amount.clone(),
            expires: None,
        };

        let allow_bin = to_json_binary(&allowance_msg).unwrap();
        let allow_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: params.cw20_address.to_string(),
            msg: allow_bin,
            funds: vec![],
        });
        msgs.push(allow_msg);

        let quote_token = QUOTE_TOKEN.load(deps.storage, payload.channel_id.unwrap())?;
        let wasm_msg: WasmMsg = utils::protocol::ucs03_transfer(
            env.block.time,
            params.ucs03_relay_contract,
            payload.channel_id.unwrap(),
            Bytes::from_str(payload.staker.as_str()).unwrap(),
            params.cw20_address.to_string(),
            payload.amount.clone(),
            Bytes::from_str(quote_token.lst_quote_token.as_str()).unwrap(),
            Uint256::from(payload.amount.clone()),
            vec![],
            H256::from_str(payload.salt.as_str()).unwrap(),
        )?;
        msgs.push(wasm_msg.into());
    } else {
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

/// Call redelegate to reward contract after withdraw reward
fn on_process_rewards(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let payload: BondRewardsPayload = from_json(msg.payload)?;
    // increment the reward balance on this contract as result of withdraw reward
    let mut reward_balance = REWARD_BALANCE.load(deps.storage)?;
    reward_balance += payload.amount;
    REWARD_BALANCE.save(deps.storage, &reward_balance)?;

    let mut msgs: Vec<CosmosMsg> = vec![];

    let attrs: Vec<Attribute> = vec![
        attr("action", "execute_split_reward"),
        attr("reward_contract", params.reward_address.clone().to_string()),
    ];

    if payload.amount != Uint128::zero() {
        let msg = ExecuteRewardMsg::SplitReward {};
        let msg_bin = to_json_binary(&msg)?;
        let redelegate_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: params.reward_address.to_string(),
            msg: msg_bin,
            funds: vec![],
        });

        msgs.push(redelegate_msg);
    }

    let res: Response = Response::new().add_messages(msgs).add_attributes(attrs);

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
    let execute_burn = cw20::Cw20ExecuteMsg::Transfer { recipient, amount };
    let the_bin = to_json_binary(&execute_burn).unwrap();
    let transfer_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw20_address,
        msg: the_bin,
        funds: vec![],
    });
    Ok(transfer_msg)
}
