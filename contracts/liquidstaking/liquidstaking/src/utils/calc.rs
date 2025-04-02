use cosmwasm_std::{Decimal, QuerierWrapper, StdResult, Uint128, Uint256};
use cw20::TokenInfoResponse;
use std::str::FromStr;

use crate::ContractError;

const DECIMAL_FRACTIONAL: u128 = 1_000_000_000_000_000_000u128;

/// return how much staking token from underlying native coin denom
pub fn calculate_staking_token_from_rate(stake_amount: Uint128, exchange_rate: Decimal) -> Uint128 {
    (Decimal::from_ratio(stake_amount, Uint128::one()) / exchange_rate).to_uint_floor()
}

/// return how much underlying native coin denom from staking token base on exchange rate
pub fn calculate_native_token_from_staking_token(
    staking_token: Uint128,
    exchange_rate: Decimal,
) -> Uint128 {
    let decimal_fract = Decimal::new(Uint128::from(DECIMAL_FRACTIONAL));
    let output =
        (exchange_rate * decimal_fract) * Decimal::from_ratio(staking_token, Uint128::one());
    output.to_uint_floor()
}

pub fn to_uint128(v: Uint256) -> StdResult<Uint128> {
    Uint128::from_str(&v.to_string())
}

pub fn total_lst_supply(
    querier: QuerierWrapper,
    cw20_address: String,
) -> Result<Uint128, ContractError> {
    let resp: TokenInfoResponse =
        querier.query_wasm_smart(cw20_address, &cw20::Cw20QueryMsg::TokenInfo {})?;
    Ok(resp.total_supply)
}

pub fn check_slippage(
    output_amount: Uint128,
    expected_amount: Uint128,
    slippage: Decimal,
) -> Result<(), ContractError> {
    let slippage_amount =
        (Decimal::from_ratio(expected_amount, Uint128::one()) * slippage).to_uint_floor();

    let min_amount = expected_amount - slippage_amount;
    let max_amount = expected_amount + slippage_amount;
    if output_amount < min_amount || output_amount > max_amount {
        return Err(ContractError::SlippageError {
            output_amount,
            min_amount,
            max_amount,
        });
    }

    Ok(())
}

pub fn calculate_query_bounds(min: Option<u64>, max: Option<u64>) -> (u64, u64) {
    let max_dist = 50;
    let min_bound = min.unwrap_or(1);
    let max_bound = match max {
        Some(max) => max.min(min.unwrap_or_default() + max_dist),
        None => min.unwrap_or_default() + max_dist,
    };
    (min_bound, max_bound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_query_bounds() {
        assert_eq!(calculate_query_bounds(None, None), (1, 50));
        assert_eq!(calculate_query_bounds(Some(200), None), (200, 250));
        assert_eq!(calculate_query_bounds(None, Some(200)), (1, 50));
        assert_eq!(calculate_query_bounds(Some(100), Some(300)), (100, 150));
        assert_eq!(calculate_query_bounds(Some(2), Some(10)), (2, 10));
        assert_eq!(calculate_query_bounds(Some(1000), Some(2000)), (1000, 1050));
        assert_eq!(calculate_query_bounds(Some(200), Some(210)), (200, 210));
    }
}
