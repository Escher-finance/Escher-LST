use crate::{msg::Ucs03ExecuteMsg, utils::protocol::*};
use cosmwasm_std::{from_json, Coin, Timestamp, Uint128, Uint256, WasmMsg};
use unionlabs_primitives::{Bytes, H256};

#[test]
fn test_ucs03_transfer() {
    let ucs03_contract_addr = "ucs03".to_string();
    let ucs03_funds = Vec::from([Coin::new(Uint128::new(100), "denom")]);
    let result = ucs03_transfer(
        Timestamp::default(),
        ucs03_contract_addr.clone(),
        u32::default(),
        Bytes::default(),
        String::default(),
        Uint128::default(),
        Bytes::default(),
        Uint256::default(),
        ucs03_funds.clone(),
        H256::default(),
    );
    if let WasmMsg::Execute {
        contract_addr,
        msg,
        funds,
    } = result.unwrap()
    {
        assert_eq!(contract_addr, ucs03_contract_addr);
        assert_eq!(funds, ucs03_funds);
        from_json::<Ucs03ExecuteMsg>(msg).unwrap();
    } else {
        panic!("not WasmMsg::Execute");
    }
}
