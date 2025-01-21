use cosmwasm_std::{
    Decimal, DelegationTotalRewardsResponse, QuerierWrapper, StdResult, Uint128, Uint256,
};
use cw20::TokenInfoResponse;

use std::str::FromStr;

use crate::{
    msg::{BondData, UnbondData},
    ContractError,
};

use crate::state::Validator;

const DECIMAL_FRACTIONAL: u128 = 1_000_000_000_000_000_000u128;

/// return how much staking token from underlying native coin denom
pub fn calculate_staking_token_from_rate(stake_amount: Uint128, exchange_rate: Decimal) -> Uint128 {
    let decimal_fract = Decimal::new(Uint128::from(DECIMAL_FRACTIONAL * DECIMAL_FRACTIONAL));
    let fract = (exchange_rate * decimal_fract).to_uint_ceil();
    Decimal::from_ratio(Uint128::from(DECIMAL_FRACTIONAL) * stake_amount, fract).to_uint_floor()
}

/// return how much underlying native coin denom from staking token base on exchange rate
pub fn calculate_native_token_from_staking_token(
    staking_token: Uint128,
    exchange_rate: Decimal,
) -> Uint128 {
    let decimal_fract = Decimal::new(Uint128::from(DECIMAL_FRACTIONAL * DECIMAL_FRACTIONAL));
    let fract = (exchange_rate * decimal_fract).to_uint_ceil();
    Decimal::from_ratio(fract * staking_token, Uint128::from(DECIMAL_FRACTIONAL)).to_uint_floor()
}

/// get total delegated token value from validators in native token
pub fn get_actual_total_delegated(
    querier: QuerierWrapper,
    delegator: String,
    denom: String,
    validators: Vec<String>,
) -> Uint128 {
    let delegations_resp = querier.query_all_delegations(delegator);
    let mut total: Uint128 = Uint128::new(0);

    if delegations_resp.is_ok() {
        total = delegations_resp
            .unwrap()
            .into_iter()
            .filter(|d| {
                d.amount.denom == denom
                    && !d.amount.amount.is_zero()
                    && validators.contains(&d.validator)
            })
            .map(|d| d.amount.amount)
            .sum();
    }

    total
}

/// get total delegated token value from validators in native token
pub fn get_actual_total_reward(
    querier: QuerierWrapper,
    delegator: String,
    denom: String,
    validators: Vec<String>,
) -> StdResult<Uint128> {
    let mut total_rewards = Uint128::new(0);
    let result: StdResult<DelegationTotalRewardsResponse> =
        querier.query_delegation_total_rewards(delegator);

    if result.is_ok() {
        for delegator_reward in result.unwrap().rewards {
            if validators.contains(&delegator_reward.validator_address) {
                for reward in delegator_reward.reward {
                    if reward.denom == denom {
                        let reward_val = to_uint128(reward.amount.to_uint_floor())?;
                        total_rewards += reward_val;
                    }
                }
            }
        }
    }

    Ok(total_rewards)
}

pub fn to_uint128(v: Uint256) -> StdResult<Uint128> {
    Uint128::from_str(&v.to_string())
}

pub fn get_liquidity_data(
    querier: QuerierWrapper,
    delegator: String,
    coin_denom: String,
    validators_list: Vec<String>,
    total_lst_supply: Uint128,
) -> Result<(Uint128, Uint128, Decimal), ContractError> {
    let delegated_amount = get_actual_total_delegated(
        querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list.clone(),
    );

    let reward = get_actual_total_reward(
        querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list,
    )?;

    let mut current_exchange_rate = Decimal::one();

    let total_bond_amount = delegated_amount + reward;

    if !delegated_amount.is_zero() && total_lst_supply.is_zero() {
        current_exchange_rate = Decimal::from_ratio(total_bond_amount, total_lst_supply);
    }

    Ok((delegated_amount, reward, current_exchange_rate))
}

pub fn total_lst_supply(
    querier: QuerierWrapper,
    cw20_address: String,
) -> Result<Uint128, ContractError> {
    let resp: TokenInfoResponse =
        querier.query_wasm_smart(cw20_address, &cw20::Cw20QueryMsg::TokenInfo {})?;
    Ok(resp.total_supply)
}

pub fn bond_calculation(
    querier: QuerierWrapper,
    delegator: String,
    amount: Uint128,
    coin_denom: String,
    validators: Vec<Validator>,
    cw20_address: Option<String>,
) -> Result<BondData, ContractError> {
    let validators_list: Vec<String> = validators.iter().map(|v| v.address.clone()).collect();

    let total_supply = match cw20_address {
        Some(cw20) => total_lst_supply(querier, cw20)?,
        None => todo!(),
    };
    let (delegated_amount, reward, exchange_rate) = get_liquidity_data(
        querier,
        delegator,
        coin_denom,
        validators_list,
        total_supply,
    )?;

    let mint_amount = calculate_staking_token_from_rate(amount, exchange_rate);

    Ok(BondData {
        mint_amount,
        delegated_amount,
        reward,
        exchange_rate,
        total_supply,
    })
}

pub fn unbond_calculation(
    querier: QuerierWrapper,
    delegator: String,
    amount: Uint128,
    coin_denom: String,
    validators: Vec<Validator>,
    cw20_address: Option<String>,
) -> Result<UnbondData, ContractError> {
    let validators_list: Vec<String> = validators.iter().map(|v| v.address.clone()).collect();

    let total_supply = match cw20_address {
        Some(cw20) => total_lst_supply(querier, cw20)?,
        None => todo!(),
    };
    let (delegated_amount, reward, exchange_rate) = get_liquidity_data(
        querier,
        delegator,
        coin_denom,
        validators_list,
        total_supply,
    )?;

    let undelegate_amount = calculate_native_token_from_staking_token(amount, exchange_rate);

    Ok(UnbondData {
        undelegate_amount,
        delegated_amount,
        reward,
        exchange_rate,
        total_supply,
    })
}
