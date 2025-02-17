use cosmwasm_std::{Decimal, StdResult, Uint128, Uint256};
use std::str::FromStr;

// ----------------------------------------
// Your calculation functions
// ----------------------------------------
const DECIMAL_FRACTIONAL: u128 = 1_000_000_000_000_000_000u128;

/// Returns how many *staking tokens* correspond to a given `stake_amount`
/// of the underlying token, given an `exchange_rate`.
pub fn calculate_staking_token_from_rate(stake_amount: Uint128, exchange_rate: Decimal) -> Uint128 {
    // Let's add some print statements to see what's happening:
    println!(
        "[calculate_staking_token_from_rate] stake_amount = {}, exchange_rate = {}",
        stake_amount, exchange_rate
    );

    // Convert stake_amount to a Decimal, then divide by exchange_rate, then floor.
    let stake_decimal = Decimal::from_ratio(stake_amount, Uint128::one());
    println!("[calculate_staking_token_from_rate] stake_decimal = {}", stake_decimal);

    let result = (stake_decimal / exchange_rate).to_uint_floor();
    println!("[calculate_staking_token_from_rate] result (floored) = {}", result);

    result
}

/// Returns how many *underlying native tokens* correspond to a given `staking_token`
/// amount, based on the current `exchange_rate`.
pub fn calculate_native_token_from_staking_token(
    staking_token: Uint128,
    exchange_rate: Decimal,
) -> Uint128 {
    println!(
        "[calculate_native_token_from_staking_token] staking_token = {}, exchange_rate = {}",
        staking_token, exchange_rate
    );

    // Multiply exchange_rate by a high-precision factor
    let decimal_fract = Decimal::new(Uint128::from(DECIMAL_FRACTIONAL));
    println!("[calculate_native_token_from_staking_token] decimal_fract = {}", decimal_fract);

    let scaled_rate = exchange_rate * decimal_fract;
    println!("[calculate_native_token_from_staking_token] scaled_rate = {}", scaled_rate);

    let staking_decimal = Decimal::from_ratio(staking_token, Uint128::one());
    println!("[calculate_native_token_from_staking_token] staking_decimal = {}", staking_decimal);

    let output = scaled_rate * staking_decimal;
    println!("[calculate_native_token_from_staking_token] output (Decimal) = {}", output);

    let floored = output.to_uint_floor();
    println!("[calculate_native_token_from_staking_token] floored = {}", floored);

    floored
}

/// Converts `Uint256` to `Uint128`, returning an error if it doesn't fit.
pub fn to_uint128(v: Uint256) -> StdResult<Uint128> {
    println!("[to_uint128] input Uint256 = {}", v);
    let text = v.to_string();
    println!("[to_uint128] as string = {}", text);

    let converted = Uint128::from_str(&text)?;
    println!("[to_uint128] converted to Uint128 = {}", converted);

    Ok(converted)
}

// ----------------------------------------
// Example unit tests
// ----------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Decimal;

    #[test]
    fn test_calculate_staking_token_from_rate() {
        println!("--- test_calculate_staking_token_from_rate START ---");
        // 1) exchange_rate = 1.0 => 1:1
        let stake_amount = Uint128::from(1000u128);
        let exchange_rate = Decimal::one();
        let staking_tokens = calculate_staking_token_from_rate(stake_amount, exchange_rate);
        assert_eq!(staking_tokens, Uint128::from(1000u128));

        // 2) exchange_rate = 2.0 => 1000 natives => 500 staking tokens
        let stake_amount = Uint128::from(1000u128);
        let exchange_rate = Decimal::from_ratio(2u128, 1u128); // 2.0
        let staking_tokens = calculate_staking_token_from_rate(stake_amount, exchange_rate);
        assert_eq!(staking_tokens, Uint128::from(500u128));

        // 3) exchange_rate = 1.3333... => 1000 / 1.3333... ~ 750
        let stake_amount = Uint128::from(1000u128);
        let exchange_rate = Decimal::from_ratio(4u128, 3u128); // 1.3333...
        let staking_tokens = calculate_staking_token_from_rate(stake_amount, exchange_rate);
        assert_eq!(staking_tokens, Uint128::from(750u128));
        println!("--- test_calculate_staking_token_from_rate END ---\n");
    }

    #[test]
    fn test_calculate_native_token_from_staking_token() {
        println!("--- test_calculate_native_token_from_staking_token START ---");
        // 1) exchange_rate = 1.0 => 1000 staking => 1000 natives
        let staking_token = Uint128::from(1000u128);
        let exchange_rate = Decimal::one();
        let natives = calculate_native_token_from_staking_token(staking_token, exchange_rate);
        assert_eq!(natives, Uint128::from(1000u128));

        // 2) exchange_rate = 0.5 => 1000 staking => 500 natives
        let staking_token = Uint128::from(1000u128);
        let exchange_rate = Decimal::from_ratio(1u128, 2u128); // 0.5
        let natives = calculate_native_token_from_staking_token(staking_token, exchange_rate);
        assert_eq!(natives, Uint128::from(500u128));

        // 3) exchange_rate = 1.5 => 1000 staking => 1500 natives
        let staking_token = Uint128::from(1000u128);
        let exchange_rate = Decimal::from_ratio(3u128, 2u128); // 1.5
        let natives = calculate_native_token_from_staking_token(staking_token, exchange_rate);
        assert_eq!(natives, Uint128::from(1500u128));
        println!("--- test_calculate_native_token_from_staking_token END ---\n");
    }

    #[test]
    fn test_to_uint128() {
        println!("--- test_to_uint128 START ---");
        // 1) normal conversion
        let v_256 = Uint256::from(12345u128);
        let v_128 = to_uint128(v_256).unwrap();
        assert_eq!(v_128, Uint128::from(12345u128));

        // 2) largest possible 128-bit value
        let max_128 = Uint256::from(u128::MAX);
        let converted = to_uint128(max_128).unwrap();
        assert_eq!(converted.u128(), u128::MAX);

        // 3) if the number is bigger than 2^128 - 1, it should fail
        // (Uncomment to test overflow behavior)
        // let too_large = Uint256::from_str("340282366920938463463374607431768211456").unwrap(); // 2^128
        // assert!(to_uint128(too_large).is_err());

        println!("--- test_to_uint128 END ---\n");
    }
}
