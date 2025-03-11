use crate::state::{BurnQueue, MintQueue, SupplyQueue};
use crate::ContractError;
use cosmwasm_std::{Decimal, QuerierWrapper, StdResult, Timestamp, Uint128, Uint256};
use cw20::TokenInfoResponse;
use std::str::FromStr;

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

pub fn get_last_epoch_in_seconds(time_in_secs: u64, epoch_period: u64) -> u64 {
    let remainder = time_in_secs % epoch_period;
    time_in_secs - remainder
}

fn get_elements_by_indices<T: Clone>(vec: &Vec<T>, indices: &[usize]) -> Vec<T> {
    let mut result = Vec::with_capacity(indices.len());

    for &index in indices {
        if index < vec.len() {
            result.push(vec[index].clone());
        }
    }

    result
}

pub fn normalize_supply_queue(supply_queue: &mut SupplyQueue, current_time: Timestamp) {
    let current_time_in_secs = current_time.seconds();
    let last_epoch_time_in_secs =
        get_last_epoch_in_seconds(current_time_in_secs, supply_queue.epoch_period);

    println!("last_epoch_time_in_secs :{}", last_epoch_time_in_secs);

    let mut mint_retain: Vec<usize> = vec![];
    for (pos, mint) in supply_queue.mint.iter().enumerate() {
        if mint.time.seconds() > last_epoch_time_in_secs {
            mint_retain.push(pos);
        }
    }

    println!("mint_retain :{:?}", mint_retain);

    let mut burn_retain: Vec<usize> = vec![];
    for (pos, burn) in supply_queue.burn.iter().enumerate() {
        if burn.time.seconds() > last_epoch_time_in_secs {
            burn_retain.push(pos)
        }
    }

    println!("burn_retain :{:?}", burn_retain);

    supply_queue.mint = get_elements_by_indices(&mut supply_queue.mint, &mint_retain);
    supply_queue.burn = get_elements_by_indices(&mut supply_queue.burn, &burn_retain);
}

pub fn normalized_total_supply(
    current_supply: Uint128,
    mint_queue: &Vec<MintQueue>,
    burn_queue: &Vec<BurnQueue>,
) -> Uint128 {
    let mut new_supply = current_supply;
    for mint in mint_queue {
        new_supply -= mint.amount;
    }
    for burn in burn_queue {
        new_supply += burn.amount;
    }
    new_supply
}

/// return how much is the exchange rate
pub fn calculate_exchange_rate(
    total_bond_amount: Uint128,
    total_supply: Uint128,
    queue: &SupplyQueue,
) -> Decimal {
    let mut exchange_rate: Decimal = Decimal::one();
    if total_bond_amount != Uint128::zero() && total_supply != Uint128::zero() {
        let normalized_total_supply =
            normalized_total_supply(total_supply, &queue.mint, &queue.burn);

        exchange_rate = Decimal::from_ratio(total_bond_amount, normalized_total_supply);
    }
    exchange_rate
}

#[cfg(test)]
#[test]
fn test_normalized_supply_queue() {
    let mint_queue = vec![
        MintQueue {
            amount: Uint128::new(400),
            time: Timestamp::from_seconds(1741685408),
        },
        MintQueue {
            amount: Uint128::new(500),
            time: Timestamp::from_seconds(1711685410),
        },
        MintQueue {
            amount: Uint128::new(200),
            time: Timestamp::from_seconds(1741685408),
        },
    ];

    let burn_queue = vec![
        BurnQueue {
            amount: Uint128::new(100),
            time: Timestamp::from_seconds(1741685408),
        },
        BurnQueue {
            amount: Uint128::new(200),
            time: Timestamp::from_seconds(1711685410),
        },
        BurnQueue {
            amount: Uint128::new(300),
            time: Timestamp::from_seconds(1741685408),
        },
    ];

    let mut supply_queue = SupplyQueue {
        mint: mint_queue,
        burn: burn_queue,
        epoch_period: 3600,
    };

    let current_time = Timestamp::from_seconds(1741685410);
    normalize_supply_queue(&mut supply_queue, current_time);
    println!(">> new_supply_queue::: {:?} ", supply_queue);
}

#[test]
fn test_normalized_total_supply() {
    let mint_queue = vec![
        MintQueue {
            amount: Uint128::new(400),
            time: Timestamp::from_seconds(1741685408),
        },
        MintQueue {
            amount: Uint128::new(500),
            time: Timestamp::from_seconds(1711685410),
        },
        MintQueue {
            amount: Uint128::new(200),
            time: Timestamp::from_seconds(1741685408),
        },
    ];

    let burn_queue = vec![
        BurnQueue {
            amount: Uint128::new(100),
            time: Timestamp::from_seconds(1741685408),
        },
        BurnQueue {
            amount: Uint128::new(200),
            time: Timestamp::from_seconds(1711685410),
        },
        BurnQueue {
            amount: Uint128::new(300),
            time: Timestamp::from_seconds(1741685408),
        },
    ];

    let mut supply_queue = SupplyQueue {
        mint: mint_queue,
        burn: burn_queue,
        epoch_period: 3600,
    };

    let current_supply = Uint128::from(10000u128);
    let current_time = Timestamp::from_seconds(1741685410);

    normalize_supply_queue(&mut supply_queue, current_time);

    let new_supply =
        normalized_total_supply(current_supply, &supply_queue.mint, &supply_queue.burn);
    println!(
        "current_supply :{} >> new_supply::: {} ",
        current_supply, new_supply
    );
}
