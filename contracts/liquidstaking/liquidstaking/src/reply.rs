use std::str::FromStr;

use crate::error::ContractError;
use crate::msg::{BondRewardsPayload, ExecuteRewardMsg, MintTokensPayload};
use crate::state::{Balance, Parameters, BALANCE, LOG, PARAMETERS};
use crate::utils;
use cosmwasm_std::{
    attr, entry_point, from_json, to_json_binary, Attribute, BankMsg, Coin, CosmosMsg, DepsMut,
    Env, Reply, Response, Uint128, Uint256, WasmMsg,
};
use unionlabs_primitives::{Bytes, H256};
pub const MINT_TOKENS_REPLY_ID: u64 = 123;
pub const MINT_CW20_TOKENS_REPLY_ID: u64 = 124;
pub const BOND_WITHDRAW_REWARD_REPLY_ID: u64 = 125;

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if !msg.result.is_ok() {
        let err = msg.result.unwrap_err();
        return Err(ContractError::ReplyError {
            message: err.to_string(),
        });
    }

    match msg.id {
        MINT_TOKENS_REPLY_ID => on_mint_tokens(deps, env, msg),
        MINT_CW20_TOKENS_REPLY_ID => on_mint_cw20_tokens(deps, env, msg),
        BOND_WITHDRAW_REWARD_REPLY_ID => on_bond_rewards(deps, env, msg),
        _ => Ok(Response::new()),
    }
}

fn on_mint_cw20_tokens(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let params: Parameters = PARAMETERS.load(deps.storage)?;
    let payload: MintTokensPayload = from_json(msg.payload)?;

    let staker_balance = deps.querier.query_balance(
        payload.staker.to_string(),
        params.liquidstaking_denom.clone(),
    )?;

    let res: Response = Response::new()
        .add_attribute("action", "mint_cw20")
        .add_attribute("receiver", payload.staker.to_string())
        .add_attribute("amount", payload.amount.to_string())
        .add_attribute("denom", params.liquidstaking_denom)
        .add_attribute("staked_token_balance", staker_balance.amount.to_string());
    Ok(res)
}

fn on_mint_tokens(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    let params: Parameters = PARAMETERS.load(deps.storage)?;
    let lst_balance = deps.querier.query_balance(
        env.contract.address.to_string(),
        params.liquidstaking_denom.clone(),
    )?;

    let balance = Balance {
        amount: lst_balance.amount,
        last_updated: env.block.time.nanos(),
    };
    BALANCE.save(deps.storage, &balance)?;

    let responses = msg.result.unwrap().msg_responses;
    let mut log = format!("responses_count: {} ", responses.len());
    for response in responses {
        log += format!("{} ", &response.type_url).as_str();
    }

    let payload: MintTokensPayload = from_json(msg.payload)?;
    log += format!("transfer to: {} amount: {}", payload.staker, payload.amount).as_str();
    LOG.save(deps.storage, &log)?;

    // if the sender is not equal as the staker, means it is from other chain
    if payload.sender == payload.staker {
        return send(deps, payload.amount, payload.staker);
    }

    // transfer to evm/bera
    return transfer(deps, env, payload.amount, payload.staker, payload.salt);
}

pub fn transfer(
    deps: DepsMut,
    env: Env,
    amount: Uint128,
    receiver: String,
    salt: String,
) -> Result<Response, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;

    let coin_amount = Coin {
        amount,
        denom: params.liquidstaking_denom.clone(),
    };

    let funds = vec![coin_amount.clone()];
    let wasm_msg: WasmMsg = utils::send_to_evm(
        env,
        params.ucs03_relay_contract,
        params.ucs03_channel.parse::<u32>().unwrap(),
        Bytes::from_str(receiver.as_str()).unwrap(),
        params.liquidstaking_denom.clone(),
        amount,
        Bytes::from_str(params.liquidstaking_denom.as_str()).unwrap(),
        Uint256::from(0u64),
        funds,
        H256::from_str(&salt).unwrap(),
    )?;

    let msg: CosmosMsg = CosmosMsg::Wasm(wasm_msg);

    let res: Response = Response::new()
        .add_message(msg)
        .add_attribute("action", "transfer")
        .add_attribute("receiver", receiver.to_string())
        .add_attribute("amount", coin_amount.amount.to_string())
        .add_attribute("denom", coin_amount.denom);
    Ok(res)
}

fn on_bond_rewards(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let payload: BondRewardsPayload = from_json(msg.payload)?;
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

pub fn send(deps: DepsMut, amount: Uint128, receiver: String) -> Result<Response, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;

    let coin_amount = Coin {
        amount,
        denom: params.underlying_coin_denom,
    };

    let bank_msg: BankMsg = BankMsg::Send {
        to_address: receiver.clone(),
        amount: vec![coin_amount.clone()],
    };

    let msg: CosmosMsg = CosmosMsg::Bank(bank_msg);

    let res: Response = Response::new()
        .add_message(msg)
        .add_attribute("action", "send")
        .add_attribute("receiver", receiver.to_string())
        .add_attribute("amount", coin_amount.amount.to_string())
        .add_attribute("denom", coin_amount.denom);
    Ok(res)
}
