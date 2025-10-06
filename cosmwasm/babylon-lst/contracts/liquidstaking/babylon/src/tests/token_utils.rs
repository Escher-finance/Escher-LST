use cosmwasm_std::{Addr, Binary, CosmosMsg, ReplyOn, Uint128, WasmMsg, from_json};

use crate::{reply::MINT_CW20_TOKENS_REPLY_ID, utils::token::*};

#[test]
fn test_get_staked_token_submsg() {
    let staker = "staker".to_string();
    let mint_amount = Uint128::new(100);
    let payload_bin = Binary::default();
    let cw20_address = Addr::unchecked("cw20");
    let submsg = get_staked_token_submsg(
        staker.clone(),
        mint_amount,
        String::default(),
        payload_bin.clone(),
        cw20_address.clone(),
    );
    assert_eq!(submsg.reply_on, ReplyOn::Always);
    assert_eq!(submsg.payload, payload_bin);
    assert_eq!(submsg.id, MINT_CW20_TOKENS_REPLY_ID);
    let CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg,
        funds,
    }) = submsg.msg
    else {
        panic!("bad cosmos msg");
    };
    assert_eq!(contract_addr, cw20_address.to_string());
    assert!(funds.is_empty());

    let cw20_msg: cw20::Cw20ExecuteMsg = from_json(msg).unwrap();

    let cw20::Cw20ExecuteMsg::Mint { recipient, amount } = cw20_msg else {
        panic!("bad cw20 msg")
    };

    assert_eq!(recipient, staker);
    assert_eq!(amount, mint_amount);
}

#[test]
fn test_burn_token() {
    let burn_amount = Uint128::new(100);
    let cw20_address = "cw20".to_string();
    let msg = burn_token(burn_amount, cw20_address.clone());
    let CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg,
        funds,
    }) = msg
    else {
        panic!("bad cosmos msg");
    };
    assert_eq!(contract_addr, cw20_address.clone());
    assert!(funds.is_empty());

    let cw20_msg: cw20::Cw20ExecuteMsg = from_json(msg).unwrap();
    let cw20::Cw20ExecuteMsg::Burn { amount } = cw20_msg else {
        panic!("bad cw20 msg");
    };
    assert_eq!(amount, burn_amount);
}
