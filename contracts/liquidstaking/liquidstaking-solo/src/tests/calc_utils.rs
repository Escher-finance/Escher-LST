use crate::{
    state::{BurnQueue, MintQueue, SupplyQueue},
    utils::calc::*,
};
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

#[test]
fn test_normalize_supply_queue() {
    let mint_queue = vec![
        MintQueue {
            amount: Uint128::new(40),
            block: 700,
        },
        MintQueue {
            amount: Uint128::new(50),
            block: 650,
        },
        MintQueue {
            amount: Uint128::new(20),
            block: 730,
        },
    ];

    let burn_queue = vec![
        BurnQueue {
            amount: Uint128::new(10),
            block: 700,
        },
        BurnQueue {
            amount: Uint128::new(20),
            block: 730,
        },
        BurnQueue {
            amount: Uint128::new(30),
            block: 650,
        },
    ];

    let mut supply_queue = SupplyQueue {
        mint: mint_queue,
        burn: burn_queue,
        epoch_period: 3600,
    };

    let current_block = 740;
    normalize_supply_queue(&mut supply_queue, current_block);
    println!(">> new_supply_queue::: {:?} ", supply_queue);
}

#[test]
fn test_normalize_total_supply() {
    let mint_queue = vec![
        MintQueue {
            amount: Uint128::new(40),
            block: 700,
        },
        MintQueue {
            amount: Uint128::new(50),
            block: 171168541,
        },
        MintQueue {
            amount: Uint128::new(20),
            block: 700,
        },
    ];

    let burn_queue = vec![
        BurnQueue {
            amount: Uint128::new(10),
            block: 700,
        },
        BurnQueue {
            amount: Uint128::new(20),
            block: 171168541,
        },
        BurnQueue {
            amount: Uint128::new(30),
            block: 700,
        },
    ];

    let mut supply_queue = SupplyQueue {
        mint: mint_queue,
        burn: burn_queue,
        epoch_period: 360,
    };

    let current_supply = Uint128::from(20000u128);
    let current_block = 1000;

    normalize_supply_queue(&mut supply_queue, current_block);

    let new_supply = normalize_total_supply(current_supply, &supply_queue.mint, &supply_queue.burn);
    println!(
        "current_supply :{} >> new_supply::: {} ",
        current_supply, new_supply
    );
}

#[test]
fn test_calculate_dust_distribution() {
    assert!(calculate_dust_distribution(Uint128::zero(), Uint128::zero()).is_empty());
    assert!(calculate_dust_distribution(Uint128::new(1000), Uint128::zero()).is_empty());
    assert_eq!(
        calculate_dust_distribution(Uint128::zero(), Uint128::new(10)).len(),
        10
    );
    assert!(
        calculate_dust_distribution(Uint128::zero(), Uint128::new(10))
            .iter()
            .all(|d| d.is_zero())
    );
    assert_eq!(
        calculate_dust_distribution(Uint128::new(10), Uint128::new(2)),
        Vec::from([Uint128::new(5), Uint128::new(5)])
    );
    assert_eq!(
        calculate_dust_distribution(Uint128::new(9), Uint128::new(2)),
        Vec::from([Uint128::new(5), Uint128::new(4)])
    );
    assert_eq!(
        calculate_dust_distribution(Uint128::new(11), Uint128::new(5)),
        Vec::from([
            Uint128::new(3),
            Uint128::new(2),
            Uint128::new(2),
            Uint128::new(2),
            Uint128::new(2),
        ])
    );
    let big_dust_amount = Uint128::new(12340123203498754234792834);
    assert_eq!(
        calculate_dust_distribution(big_dust_amount, Uint128::new(1500))
            .iter()
            .sum::<Uint128>(),
        big_dust_amount
    );
}
