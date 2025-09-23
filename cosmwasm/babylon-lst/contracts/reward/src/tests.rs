use std::str::FromStr;

use cosmwasm_std::{Decimal, Uint128};

use crate::helpers::split_revenue;

#[test]
fn test_split_revenue() {
    let reward_amount = Uint128::new(251);
    let fee_rate = Decimal::from_str("0.1").unwrap();

    //check Decimal(100000000000000000)
    println!("fee_rate: {fee_rate:?}");
    let (restake, fee) = split_revenue(reward_amount, fee_rate, "stake".into());
    println!(
        "split_revenue: {reward_amount}, restake: {restake}, fee: {fee}"
    );
}
