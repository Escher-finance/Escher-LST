use cosmwasm_std::{Decimal, QuerierWrapper, StdResult, Uint128, Uint256};
use cw20::TokenInfoResponse;
use std::str::FromStr;

use crate::ContractError;

/// return how much staking token from underlying native coin denom
pub fn calculate_staking_token_from_rate(stake_amount: Uint128, exchange_rate: Decimal) -> Uint128 {
    (Decimal::from_ratio(stake_amount, Uint128::one()) / exchange_rate).to_uint_floor()
}

/// return how much underlying native coin denom from staking token base on exchange rate
pub fn calculate_native_token_from_staking_token(
    staking_token: Uint128,
    exchange_rate: Decimal,
) -> Uint128 {
    (exchange_rate * Decimal::from_ratio(staking_token, Uint128::one())).to_uint_floor()
}

/// Convert Uint256 to Uint128
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
