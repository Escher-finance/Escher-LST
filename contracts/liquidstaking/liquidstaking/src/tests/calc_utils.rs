use crate::utils::calc::*;
use cosmwasm_std::{
    from_json, testing::MockQuerier, to_json_binary, Decimal, Empty, QuerierWrapper, SystemError,
    SystemResult, Uint128, Uint256,
};
use cw20::TokenInfoResponse;
use std::str::FromStr;

#[test]
fn test_calculate_query_bounds() {
    assert_eq!(calculate_query_bounds(None, None), (1, 50));
    assert_eq!(calculate_query_bounds(Some(200), None), (200, 249));
    assert_eq!(calculate_query_bounds(None, Some(200)), (1, 50));
    assert_eq!(calculate_query_bounds(Some(100), Some(300)), (100, 149));
    assert_eq!(calculate_query_bounds(Some(2), Some(10)), (2, 10));
    assert_eq!(calculate_query_bounds(Some(1000), Some(2000)), (1000, 1049));
    assert_eq!(calculate_query_bounds(Some(200), Some(210)), (200, 210));
}

#[test]
fn test_calculate_staking_token_from_rate() {
    let stake_amount = Uint128::new(112382);
    assert_eq!(
        calculate_staking_token_from_rate(stake_amount, Decimal::from_ratio(1_u128, 2_u128)),
        stake_amount * Uint128::new(2)
    );
    assert_eq!(
        calculate_staking_token_from_rate(stake_amount, Decimal::from_str("1.0").unwrap()),
        stake_amount
    );
}

#[test]
fn test_calculate_native_token_from_staking_token() {
    let staking_token = Uint128::new(112382);
    assert_eq!(
        calculate_native_token_from_staking_token(
            staking_token,
            Decimal::from_ratio(1_u128, 2_u128)
        ),
        staking_token / Uint128::new(2)
    );

    let decimal_fractional: u128 = 1_000_000_000_000_000_000u128;
    let staking_token = Uint128::new(decimal_fractional);
    assert_eq!(
        calculate_native_token_from_staking_token(
            staking_token,
            Decimal::from_ratio(1u128, staking_token)
        ),
        Uint128::one()
    );
    let staking_token = Uint128::new(decimal_fractional + 1);
    assert_eq!(
        calculate_native_token_from_staking_token(
            staking_token,
            Decimal::from_ratio(1u128, staking_token)
        ),
        Uint128::zero() // Not enough precision
    );
}

#[test]
fn test_check_slippage() {
    // Same value
    assert!(check_slippage(Uint128::new(10), Uint128::new(10), Decimal::zero()).is_ok());

    // Good - lower bound
    assert!(check_slippage(Uint128::new(98), Uint128::new(100), Decimal::percent(2)).is_ok());
    // Fails - lower bound
    assert!(check_slippage(Uint128::new(98), Uint128::new(100), Decimal::percent(1)).is_err());

    // Good - upper bound
    assert!(check_slippage(Uint128::new(100), Uint128::new(105), Decimal::percent(5)).is_ok());
    // Fails - upper bound
    assert!(check_slippage(Uint128::new(100), Uint128::new(105), Decimal::percent(4)).is_err());
}

#[test]
fn test_to_uint128() {
    let amount = 123050;
    assert_eq!(
        to_uint128(Uint256::from_u128(amount)),
        Ok(Uint128::new(amount))
    )
}

#[test]
fn test_total_lst_supply() {
    let mut querier = MockQuerier::default();
    let total_supply = Uint128::new(100000);
    querier.update_wasm(move |wasm_query| {
        let unsupported_err = SystemResult::Err(SystemError::Unknown {});
        match wasm_query {
            cosmwasm_std::WasmQuery::Smart {
                contract_addr: _,
                msg,
            } => {
                let cw20_msg: cw20::Cw20QueryMsg = from_json(msg).unwrap();
                match cw20_msg {
                    cw20::Cw20QueryMsg::TokenInfo {} => {
                        let response = TokenInfoResponse {
                            name: String::default(),
                            symbol: String::default(),
                            decimals: u8::default(),
                            total_supply,
                        };
                        let bin = to_json_binary(&response).unwrap();
                        return SystemResult::Ok(cosmwasm_std::ContractResult::Ok(bin));
                    }
                    _ => unsupported_err,
                }
            }
            _ => unsupported_err,
        }
    });
    let querier_wrapper = QuerierWrapper::<Empty>::new(&querier);
    assert_eq!(
        total_lst_supply(querier_wrapper, "cw20".to_string()).unwrap(),
        total_supply
    );
}
