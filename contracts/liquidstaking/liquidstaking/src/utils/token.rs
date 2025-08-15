use cosmwasm_std::{to_json_binary, CosmosMsg, Uint128, WasmMsg};

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
