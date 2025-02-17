use cosmwasm_std::{Decimal, Uint128, Uint256};

// Move functions outside of test module so they can be used by tests
pub fn calculate_staking_token_from_rate(stake_amount: Uint128, exchange_rate: Decimal) -> Uint128 {
    println!("Calculation: {} * (1/{}) = {} * (1/{})", 
        stake_amount,
        exchange_rate,
        stake_amount.u128(),
        exchange_rate.atomics());
    let result = stake_amount.multiply_ratio(1u128, exchange_rate.atomics());
    println!("Result: {}", result);
    result
}

pub fn calculate_native_token_from_staking_token(staking_token: Uint128, exchange_rate: Decimal) -> Uint128 {
    println!("Calculation: {} * {} = {} * {}", 
        staking_token,
        exchange_rate,
        staking_token.u128(),
        exchange_rate.atomics());
    let result = staking_token.multiply_ratio(exchange_rate.atomics(), 1u128);
    println!("Result: {}", result);
    result
}

pub fn to_uint128(value: Uint256) -> Result<Uint128, String> {
    value.try_into().map_err(|_| "overflow".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_staking_token_from_rate() {
        println!("\n=== Testing calculate_staking_token_from_rate ===");
        
        // Test 1
        println!("\nTest 1: Exchange rate = 1.0");
        let stake_amount = Uint128::from(1000u128);
        let exchange_rate = Decimal::one();
        println!("Input: stake_amount = {}, exchange_rate = {}", stake_amount, exchange_rate);
        let staking_tokens = calculate_staking_token_from_rate(stake_amount, exchange_rate);
        println!("Output: staking_tokens = {}", staking_tokens);
        assert_eq!(staking_tokens, stake_amount);

        // Test 2
        println!("\nTest 2: Exchange rate = 2.0");
        let stake_amount = Uint128::from(1000u128);
        let exchange_rate = Decimal::from_ratio(2u128, 1u128);
        println!("Input: stake_amount = {}, exchange_rate = {}", stake_amount, exchange_rate);
        let staking_tokens = calculate_staking_token_from_rate(stake_amount, exchange_rate);
        println!("Output: staking_tokens = {} (1000/2 = 500)", staking_tokens);
        assert_eq!(staking_tokens, Uint128::from(500u128));

        // Test 3
        println!("\nTest 3: Exchange rate = 1.3333...");
        let stake_amount = Uint128::from(1000u128);
        let exchange_rate = Decimal::from_ratio(4u128, 3u128);
        println!("Input: stake_amount = {}, exchange_rate = {}", stake_amount, exchange_rate);
        let staking_tokens = calculate_staking_token_from_rate(stake_amount, exchange_rate);
        println!("Output: staking_tokens = {} (1000/1.3333 ≈ 750)", staking_tokens);
        assert_eq!(staking_tokens, Uint128::from(750u128));
    }

    #[test]
    fn test_calculate_native_token_from_staking_token() {
        println!("\n=== Testing calculate_native_token_from_staking_token ===");

        // Test 1
        println!("\nTest 1: Exchange rate = 1.0");
        let staking_token = Uint128::from(1000u128);
        let exchange_rate = Decimal::one();
        println!("Input: staking_token = {}, exchange_rate = {}", staking_token, exchange_rate);
        let native_amount = calculate_native_token_from_staking_token(staking_token, exchange_rate);
        println!("Output: native_amount = {}", native_amount);
        assert_eq!(native_amount, staking_token);

        // Test 2
        println!("\nTest 2: Exchange rate = 0.5");
        let staking_token = Uint128::from(1000u128);
        let exchange_rate = Decimal::from_ratio(1u128, 2u128);
        println!("Input: staking_token = {}, exchange_rate = {}", staking_token, exchange_rate);
        let native_amount = calculate_native_token_from_staking_token(staking_token, exchange_rate);
        println!("Output: native_amount = {} (1000 * 0.5 = 500)", native_amount);
        assert_eq!(native_amount, Uint128::from(500u128));

        // Test 3
        println!("\nTest 3: Exchange rate = 1.5");
        let staking_token = Uint128::from(1000u128);
        let exchange_rate = Decimal::from_ratio(3u128, 2u128);
        println!("Input: staking_token = {}, exchange_rate = {}", staking_token, exchange_rate);
        let native_amount = calculate_native_token_from_staking_token(staking_token, exchange_rate);
        println!("Output: native_amount = {} (1000 * 1.5 = 1500)", native_amount);
        assert_eq!(native_amount, Uint128::from(1500u128));
    }

    #[test]
    fn test_to_uint128() {
        // Check a normal conversion
        let v_256 = Uint256::from(12345u128);
        let v_128 = to_uint128(v_256).unwrap();
        assert_eq!(v_128, Uint128::from(12345u128));

        // Edge case: very large number still < 2^128
        let max_128 = Uint256::from(u128::MAX);
        let converted = to_uint128(max_128).unwrap();
        assert_eq!(converted.u128(), u128::MAX);

        // If you want to ensure overflow checks, you could try:
        // let too_large = Uint256::from_str("340282366920938463463374607431768211456").unwrap(); // 2^128
        // assert!(to_uint128(too_large).is_err()); // Should overflow
    }

    #[test]
    fn test_edge_cases_staking_token() {
        println!("\n=== Testing Edge Cases for Staking Token ===");

        // Test with very small exchange rate
        println!("\nTest with very small exchange rate (0.001)");
        let stake_amount = Uint128::from(1000u128);
        let exchange_rate = Decimal::from_ratio(1u128, 1000u128);
        println!("Input: stake_amount = {}, exchange_rate = {}", stake_amount, exchange_rate);
        let staking_tokens = calculate_staking_token_from_rate(stake_amount, exchange_rate);
        println!("Output: staking_tokens = {} (1000/0.001 = 1,000,000)", staking_tokens);
        assert_eq!(staking_tokens, Uint128::from(1_000_000u128));

        // Test with very large exchange rate
        println!("\nTest with very large exchange rate (1000.0)");
        let stake_amount = Uint128::from(1000u128);
        let exchange_rate = Decimal::from_ratio(1000u128, 1u128);
        println!("Input: stake_amount = {}, exchange_rate = {}", stake_amount, exchange_rate);
        let staking_tokens = calculate_staking_token_from_rate(stake_amount, exchange_rate);
        println!("Output: staking_tokens = {} (1000/1000 = 1)", staking_tokens);
        assert_eq!(staking_tokens, Uint128::from(1u128));
    }

    #[test]
    fn test_edge_cases_native_token() {
        // Test with very small exchange rate
        let staking_token = Uint128::from(1000u128);
        let exchange_rate = Decimal::from_ratio(1u128, 1000u128); // 0.001
        let native_amount = calculate_native_token_from_staking_token(staking_token, exchange_rate);
        assert_eq!(native_amount, Uint128::from(1u128));

        // Test with very large exchange rate
        let staking_token = Uint128::from(1000u128);
        let exchange_rate = Decimal::from_ratio(1000u128, 1u128); // 1000.0
        let native_amount = calculate_native_token_from_staking_token(staking_token, exchange_rate);
        assert_eq!(native_amount, Uint128::from(1_000_000u128));

        // Test with zero staking token
        let staking_token = Uint128::zero();
        let exchange_rate = Decimal::one();
        let native_amount = calculate_native_token_from_staking_token(staking_token, exchange_rate);
        assert_eq!(native_amount, Uint128::zero());
    }
}
