use std::str::FromStr;

use cosmwasm_std::{Decimal, Uint128};

use crate::{execute::*, state::QuoteToken, utils, ContractError};

#[test]
fn test_calculate_native_token() {
    let staking_token = Uint128::from(10000u32);
    //60926366
    let exchange_rate =
        Decimal::from_ratio(Uint128::from(5350444044771u128), Uint128::from(30000u128));

    println!("exchange_rate: {}", exchange_rate);

    let undelegate_amount: Uint128 =
        utils::calc::calculate_native_token_from_staking_token(staking_token, exchange_rate);
    println!("undelegate_amount: {}", undelegate_amount);
}

#[test]
fn exchange_rate_calculation() {
    let total_bond = Uint128::new(100);

    let a = Uint128::new(10);
    let b = Uint128::new(50);
    let exchange_rate = Decimal::from_ratio(a, b);
    println!("{:?} / {:?}", total_bond, exchange_rate);

    let token = utils::calc::calculate_staking_token_from_rate(total_bond, exchange_rate);

    println!("token: {:?}", token);
    assert_eq!(token, Uint128::new(500));

    // - Rewards for 4 days: 1000 Union * 0.0274% * 4 = 1.096 Union
    // - Total staked Union + rewards (U + R): 1001.096 Union
    // - Total LUnion (L): 1000 LUnion

    // - New exchange rate: 1001.096 / 1000 = 1.001096 Union per LUnion
    // - Bob receives: 500 / 1.001096 = 499.45 LUnion

    let a = Uint128::new(1001096);
    let b = Uint128::new(1000000);
    let new_exchange_rate = Decimal::from_ratio(a, b);

    let bond_amount = Uint128::new(500000000);
    let mint_amount =
        utils::calc::calculate_staking_token_from_rate(bond_amount, new_exchange_rate);
    assert_eq!(mint_amount, Uint128::new(499452599));
    println!("mint_amount: {:?}", mint_amount);
}

#[test]
fn exchange_unbond_rate_calculation() {
    let staking_token = Uint128::new(100);

    let a = Uint128::new(110);
    let b = Uint128::new(100);
    let exchange_rate = Decimal::from_ratio(a, b);

    let token =
        utils::calc::calculate_native_token_from_staking_token(staking_token, exchange_rate);
    assert_eq!(token, Uint128::new(110));
}

#[test]
fn slippage_calculation() {
    let expected = Uint128::new(10000);
    let slippage = Decimal::from_str("0.01").unwrap();
    let output = Uint128::new(10140);

    let result = utils::calc::check_slippage(output, expected, slippage);
    assert!(result.is_err());

    let output = Uint128::new(10100);
    let result = utils::calc::check_slippage(output, expected, slippage);
    assert!(result.is_ok());
}

#[test]
fn test_update_quote_token_channel_id_should_match() {
    use cosmwasm_std::testing::{mock_dependencies, mock_env};

    let mut deps = mock_dependencies();
    let env = mock_env();

    let owner = deps.api.addr_make("owner");

    let info = cosmwasm_std::MessageInfo {
        sender: owner.clone(),
        funds: vec![],
    };
    let mut channel_id = 10;
    let quote_token = QuoteToken {
        channel_id,
        quote_token: "a".to_string(),
        lst_quote_token: "b".to_string(),
    };

    cw_ownable::initialize_owner(&mut deps.storage, &deps.api, Some(owner.as_str())).unwrap();

    // Good
    update_quote_token(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        channel_id,
        quote_token.clone(),
    )
    .unwrap();

    channel_id += 1;

    // Fails - channel_id doesn't match
    let err = update_quote_token(deps.as_mut(), env, info, channel_id, quote_token).unwrap_err();
    assert!(matches!(err, ContractError::InvalidQuoteTokens {}));
}
