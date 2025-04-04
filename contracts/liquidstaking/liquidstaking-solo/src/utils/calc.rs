use crate::state::{BurnQueue, MintQueue, SupplyQueue};
use crate::ContractError;
use cosmwasm_std::{Decimal, QuerierWrapper, StdResult, Uint128, Uint256};
use cw20::TokenInfoResponse;
use std::str::FromStr;

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
    (exchange_rate * Decimal::from_ratio(staking_token, Uint128::one())).to_uint_floor()
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

pub fn get_last_epoch_block(block: u64, epoch_period: u32) -> u64 {
    let remainder: u64 = block % epoch_period as u64;
    block - remainder
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

pub fn normalize_supply_queue(supply_queue: &mut SupplyQueue, current_block: u64) {
    let last_epoch_block = get_last_epoch_block(current_block, supply_queue.epoch_period);
    let mut mint_retain: Vec<usize> = vec![];
    for (pos, mint) in supply_queue.mint.iter().enumerate() {
        if mint.block > last_epoch_block {
            mint_retain.push(pos);
        }
    }
    let mut burn_retain: Vec<usize> = vec![];
    for (pos, burn) in supply_queue.burn.iter().enumerate() {
        if burn.block > last_epoch_block {
            burn_retain.push(pos)
        }
    }
    supply_queue.mint = get_elements_by_indices(&mut supply_queue.mint, &mint_retain);
    supply_queue.burn = get_elements_by_indices(&mut supply_queue.burn, &burn_retain);
}

pub fn normalize_total_supply(
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
        let normalize_total_supply = normalize_total_supply(total_supply, &queue.mint, &queue.burn);

        exchange_rate = Decimal::from_ratio(total_bond_amount, normalize_total_supply);
    }
    exchange_rate
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
