use crate::reply::MINT_CW20_TOKENS_REPLY_ID;
use cosmwasm_std::{to_json_binary, Addr, Binary, CosmosMsg, SubMsg, Uint128, WasmMsg};

pub fn get_staked_token_submsg(
    _delegator: String,
    staker: String,
    mint_amount: Uint128,
    _liquidstaking_denom: String,
    payload_bin: Binary,
    cw20_address: Addr,
) -> SubMsg {
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

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_json, ReplyOn};

    use super::*;

    #[test]
    fn test_get_staked_token_submsg() {
        let staker = "staker".to_string();
        let mint_amount = Uint128::new(100);
        let payload_bin = Binary::default();
        let cw20_address = Addr::unchecked("cw20");
        let submsg = get_staked_token_submsg(
            String::new(),
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
        assert_eq!(contract_addr, cw20_address.to_string());
        assert!(funds.is_empty());

        let cw20_msg: cw20::Cw20ExecuteMsg = from_json(msg).unwrap();
        let cw20::Cw20ExecuteMsg::Burn { amount } = cw20_msg else {
            panic!("bad cw20 msg");
        };
        assert_eq!(amount, burn_amount);
    }
}
