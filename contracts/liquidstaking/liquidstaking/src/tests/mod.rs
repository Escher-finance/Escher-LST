#![cfg(test)]

pub mod batch_utils;
pub mod calc_utils;
pub mod delegation_utils;
pub mod execute;
pub mod protocol_utils;
pub mod query;
pub mod state;
pub mod token_utils;
pub mod validation_utils;

use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_multi_test::{App, ContractWrapper, Executor};

use crate::{
    contract::{execute, instantiate},
    msg::InstantiateMsg,
    query::query,
    state::Validator,
};

pub fn make_contract(app: &mut App, sender: Addr, instantiate_msg: InstantiateMsg) -> Addr {
    let code = ContractWrapper::new(execute, instantiate, query);
    let code_id = app.store_code(Box::new(code));
    app.instantiate_contract(code_id, sender, &instantiate_msg, &[], "contract", None)
        .unwrap()
}

pub fn mock_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        underlying_coin_denom: "denom".to_string(),
        validators: vec![
            Validator {
                address: "abc".to_string(),
                weight: 10,
            },
            Validator {
                address: "bcd".to_string(),
                weight: 20,
            },
        ],
        liquidstaking_denom: "edenom".to_string(),
        ucs03_relay_contract: String::new(),
        fee_receiver: Addr::unchecked("fee_receiver"),
        unbonding_time: 1944000,
        reward_code_id: 0,
        fee_rate: Decimal::from_ratio(Uint128::new(5), Uint128::new(100)),
        cw20_address: Addr::unchecked("cw20_address"),
        salt: "salt".to_string(),
        quote_tokens: vec![],
        batch_period: 3600,
    }
}
