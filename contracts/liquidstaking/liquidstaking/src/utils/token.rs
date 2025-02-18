use crate::reply::MINT_CW20_TOKENS_REPLY_ID;
use crate::token_factory_api::TokenFactoryMsg;
use cosmwasm_std::{to_json_binary, Addr, Binary, CosmosMsg, SubMsg, Uint128, WasmMsg};

pub fn get_staked_token_submsg(
    _delegator: String,
    staker: String,
    mint_amount: Uint128,
    _liquidstaking_denom: String,
    payload_bin: Binary,
    cw20_address: Addr,
) -> SubMsg<TokenFactoryMsg> {
    let mint = cw20::Cw20ExecuteMsg::Mint {
        recipient: staker,
        amount: mint_amount,
    };
    let mint_bin = to_json_binary(&mint).unwrap();
    let mint_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw20_address.to_string(),
        msg: mint_bin,
        funds: vec![],
    });
    let sub_msg: SubMsg<TokenFactoryMsg> =
        SubMsg::reply_always(mint_msg, MINT_CW20_TOKENS_REPLY_ID)
            .with_payload(payload_bin)
            .into();
    sub_msg
}

pub fn get_burn_msg(denom: String, burn_amount: Uint128, delegator: String) -> TokenFactoryMsg {
    let burn_msg = TokenFactoryMsg::BurnTokens {
        denom: denom.clone(),
        amount: burn_amount,
        burn_from_address: delegator,
    };

    burn_msg
}

pub fn burn_token(amount: Uint128, cw20_address: String) -> CosmosMsg<TokenFactoryMsg> {
    let execute_burn = cw20::Cw20ExecuteMsg::Burn { amount };
    let burn_bin = to_json_binary(&execute_burn).unwrap();
    let burn_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw20_address,
        msg: burn_bin,
        funds: vec![],
    });
    let msg: CosmosMsg<TokenFactoryMsg> = burn_msg.into();
    msg
}
