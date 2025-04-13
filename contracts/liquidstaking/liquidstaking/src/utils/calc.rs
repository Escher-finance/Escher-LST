use cosmwasm_std::{Decimal, QuerierWrapper, StdResult, Uint128, Uint256};
use cw20::TokenInfoResponse;
use std::str::FromStr;

use crate::ContractError;

/// return how much output if multiplied with decimal rate
pub fn calc_with_rate(input: Uint128, rate: Decimal) -> Uint128 {
    (rate * Decimal::from_ratio(input, Uint128::one())).to_uint_floor()
}

/// return how much staking token from underlying native coin denom
pub fn calculate_staking_token_from_rate(stake_amount: Uint128, exchange_rate: Decimal) -> Uint128 {
    (Decimal::from_ratio(stake_amount, Uint128::one()) / exchange_rate).to_uint_floor()
}

/// return how much fee from reward
pub fn calculate_fee_from_reward(reward: Uint128, fee_rate: Decimal) -> Uint128 {
    (fee_rate * Decimal::from_ratio(reward, Uint128::one())).to_uint_floor()
}

/// return how much underlying native coin denom from staking token base on exchange rate
pub fn calculate_native_token_from_staking_token(
    staking_token: Uint128,
    exchange_rate: Decimal,
) -> Uint128 {
    calc_with_rate(staking_token, exchange_rate)
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

pub fn calculate_query_bounds(min: Option<u64>, max: Option<u64>) -> (u64, u64) {
    // NOTE: 49 because both bounds are inclusive
    let max_dist = 49;
    let min_bound = min.unwrap_or(1);
    let max_bound = match max {
        Some(max) => max.min(min_bound + max_dist),
        None => min_bound + max_dist,
    };
    (min_bound, max_bound)
}
