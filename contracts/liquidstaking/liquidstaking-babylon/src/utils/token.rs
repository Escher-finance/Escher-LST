use cosmwasm_std::{Addr, Binary, CosmosMsg, SubMsg, Uint128, WasmMsg, to_json_binary};

use crate::reply::MINT_CW20_TOKENS_REPLY_ID;

pub fn get_staked_token_submsg(
    recipient: String,
    mint_amount: Uint128,
    _liquidstaking_denom: String,
    payload_bin: Binary,
    cw20_address: Addr,
) -> SubMsg {
    let mint = cw20::Cw20ExecuteMsg::Mint {
        recipient,
        amount: mint_amount,
    };
    let mint_bin = to_json_binary(&mint).unwrap();
    let mint_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw20_address.to_string(),
        msg: mint_bin,
        funds: vec![],
    });
    let sub_msg: SubMsg = SubMsg::reply_always(mint_msg, MINT_CW20_TOKENS_REPLY_ID)
        .with_payload(payload_bin)
        .into();
    sub_msg
}

pub fn burn_token(amount: Uint128, cw20_address: String) -> CosmosMsg {
    let execute_burn = cw20::Cw20ExecuteMsg::Burn { amount };
    let burn_bin = to_json_binary(&execute_burn).unwrap();
    let burn_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw20_address,
        msg: burn_bin,
        funds: vec![],
    });
    let msg: CosmosMsg = burn_msg.into();
    msg
}
