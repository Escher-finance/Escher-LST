use crate::reply::MINT_TOKENS_REPLY_ID;
use crate::state::Parameters;
use crate::token_factory_api::TokenFactoryMsg;
use cosmwasm_std::{Addr, Binary, CosmosMsg, SubMsg, Uint128, WasmMsg};

#[cfg(not(nonunion))]
pub fn get_staked_token_submsg(
    delegator: String,
    _staker: String,
    mint_amount: Uint128,
    liquidstaking_denom: String,
    payload_bin: Binary,
    _params: Parameters,
) -> SubMsg<TokenFactoryMsg> {
    let mint_msg = TokenFactoryMsg::MintTokens {
        denom: liquidstaking_denom,
        amount: mint_amount,
        mint_to_address: delegator.to_string(),
    };

    let sub_msg: SubMsg<TokenFactoryMsg> = SubMsg::reply_always(mint_msg, MINT_TOKENS_REPLY_ID)
        .with_payload(payload_bin)
        .into();
    sub_msg
}

#[cfg(nonunion)]
pub fn get_staked_token_submsg(
    _delegator: String,
    staker: String,
    mint_amount: Uint128,
    _liquidstaking_denom: String,
    payload_bin: Binary,
    params: Parameters,
) -> SubMsg<TokenFactoryMsg> {
    let mint = cw20::Cw20ExecuteMsg::Mint {
        recipient: staker,
        amount: mint_amount,
    };
    let mint_bin = to_json_binary(&mint).unwrap();
    let mint_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: params.cw20_address.unwrap().to_string(),
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

#[cfg(not(nonunion))]
pub fn burn_token(
    delegator: String,
    amount: Uint128,
    liquidstaking_denom: String,
    _cw20_address: Option<Addr>,
) -> CosmosMsg<TokenFactoryMsg> {
    let burn_msg = get_burn_msg(liquidstaking_denom.clone(), amount, delegator.to_string());
    let msg: CosmosMsg<TokenFactoryMsg> = burn_msg.into();
    msg
}

#[cfg(nonunion)]
pub fn burn_token(
    _delegator: String,
    amount: Uint128,
    _liquidstaking_denom: String,
    cw20_address: Option<Addr>,
) -> CosmosMsg<TokenFactoryMsg> {
    let execute_burn = cw20::Cw20ExecuteMsg::Burn { amount };
    let burn_bin = to_json_binary(&execute_burn).unwrap();
    let burn_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw20_address.unwrap().to_string(),
        msg: burn_bin,
        funds: vec![],
    });
    let msg: CosmosMsg<TokenFactoryMsg> = burn_msg.into();
    msg
}
