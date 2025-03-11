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

fn grab_by_indices<T>(vec: &mut Vec<T>, indices: Vec<usize>) -> Vec<T> {
    // Collect removed elements
    let mut grabbed = Vec::with_capacity(indices.len());

    // Remove elements at the specified indices
    for &index in &indices {
        if index < vec.len() {
            grabbed.push(vec.remove(index));
        }
    }

    // Reverse the grabbed elements to maintain original order
    grabbed.reverse();

    grabbed
}

pub fn normalize_supply_queue(supply_queue: &mut SupplyQueue, current_time: Timestamp) {
    let current_time_in_secs = current_time.seconds();
    let last_epoch_time_in_secs =
        get_last_epoch_in_seconds(current_time_in_secs, supply_queue.epoch_period);

    let mut mint_retain: Vec<usize> = vec![];
    for (pos, mint) in supply_queue.mint.iter().enumerate() {
        if mint.time.seconds() > last_epoch_time_in_secs {
            mint_retain.push(pos);
        }
    }

    let mut burn_retain: Vec<usize> = vec![];
    for (pos, burn) in supply_queue.burn.iter().enumerate() {
        if burn.time.seconds() > last_epoch_time_in_secs {
            burn_retain.push(pos)
        }
    }

    supply_queue.mint = grab_by_indices(&mut supply_queue.mint, mint_retain);
    supply_queue.burn = grab_by_indices(&mut supply_queue.burn, burn_retain);
}

pub fn normalized_total_supply(
    supply: Uint128,
    mint_queue: &Vec<MintQueue>,
    burn_queue: &Vec<BurnQueue>,
) -> Uint128 {
    let mut new_supply = supply;
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
    queue: SupplyQueue,
) -> Decimal {
    let mut exchange_rate: Decimal = Decimal::one();
    if total_bond_amount != Uint128::zero() && total_supply != Uint128::zero() {
        let normalized_total_supply =
            normalized_total_supply(total_supply, &queue.mint, &queue.burn);

        exchange_rate = Decimal::from_ratio(total_bond_amount, normalized_total_supply);
    }
    exchange_rate
}
