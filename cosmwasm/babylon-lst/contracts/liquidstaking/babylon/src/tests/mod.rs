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

use std::str::FromStr;

use cosmwasm_std::{
    testing::{MockApi, MockQuerier, MockStorage},
    Addr, Coin, Decimal, OwnedDeps, Uint128,
};
use cw_multi_test::{App, ContractWrapper, Executor};

use crate::{
    contract::{execute, instantiate},
    msg::InstantiateMsg,
    query::query,
    state::{Parameters, Validator},
};

const NATIVE_DENOM: &str = "utoken";

#[allow(clippy::missing_panics_doc)]
pub fn make_contract(app: &mut App, sender: Addr, instantiate_msg: &InstantiateMsg) -> Addr {
    let code = ContractWrapper::new(execute, instantiate, query);
    let code_id = app.store_code(Box::new(code));
    app.instantiate_contract(code_id, sender, &instantiate_msg, &[], "contract", None)
        .unwrap()
}

#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn mock_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        epoch_period: None,
        use_external_reward: None,
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
        unbonding_time: 1_944_000,
        reward_code_id: 0,
        fee_rate: Decimal::from_ratio(Uint128::new(5), Uint128::new(100)),
        cw20_address: Addr::unchecked("cw20_address"),
        salt: "salt".to_string(),
        quote_tokens: vec![],
        batch_period: 3600,
        batch_limit: 50,
        min_bond: Uint128::from_str("10000").unwrap(),
        min_unbond: Uint128::from_str("10000").unwrap(),
        transfer_fee: Uint128::from_str("20000000").unwrap(),
        transfer_handler: Addr::unchecked("transfer_handler").to_string(),
        zkgm_token_minter: Addr::unchecked("zkgm_token_minter").to_string(),
    }
}

#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn mock_parameters() -> Parameters {
    Parameters {
        underlying_coin_denom: NATIVE_DENOM.to_string(),
        liquidstaking_denom: "liquidstaking".to_string(),
        ucs03_relay_contract: Addr::unchecked("ucs03").to_string(),
        unbonding_time: u64::default(),
        cw20_address: Addr::unchecked("cw20"),
        reward_address: Addr::unchecked("reward"),
        fee_rate: Decimal::default(),
        fee_receiver: Addr::unchecked("fee"),
        batch_period: u64::default(),
        batch_limit: 50,
        min_bond: Uint128::from_str("10000").unwrap(),
        min_unbond: Uint128::from_str("10000").unwrap(),
        transfer_fee: Uint128::from(20_000_000_u128),
        transfer_handler: Addr::unchecked("transfer_handler").to_string(),
        zkgm_token_minter: Addr::unchecked("zkgm_token_minter").to_string(),
    }
}

fn setup_validators_delegation(
    mut deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    delegator_addr: &Addr,
    validators: &[Validator],
    denom: String,
    total_delegation: Uint128,
) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut the_validators: Vec<cosmwasm_std::Validator> = vec![];
    let mut delegations: Vec<cosmwasm_std::FullDelegation> = vec![];

    for validator in validators {
        the_validators.push(cosmwasm_std::Validator::create(
            validator.address.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ));

        delegations.push(cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            validator.address.clone(),
            Coin::new(
                total_delegation.multiply_ratio(validator.weight, Uint128::new(100u128)),
                denom.clone(),
            ),
            Coin::default(),
            Vec::default(),
        ));
    }

    deps.querier
        .staking
        .update(denom, the_validators.as_slice(), delegations.as_slice());

    deps
}
