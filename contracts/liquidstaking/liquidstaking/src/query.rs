use std::marker::PhantomData;

use crate::msg::{QueryMsg, StakingLiquidity};
use crate::state::unbond_record;
use crate::state::{
    Balance, Parameters, QuoteToken, State, UnbondRecord, ValidatorsRegistry, PARAMETERS,
    QUOTE_TOKEN, REWARD_BALANCE, STATE, VALIDATORS_REGISTRY,
};
use crate::utils::batch::{batches, Batch, BatchStatus};
use crate::utils::calc;
use crate::utils::calc::calculate_query_bounds;
use crate::utils::delegation::{get_actual_total_delegated, get_unclaimed_reward};
use crate::ContractError;
use cosmwasm_std::{entry_point, to_json_binary, Decimal, Order, Uint128};
use cosmwasm_std::{Binary, Deps, Env, Storage};
use cw2::ContractVersion;
use cw_ownable::get_ownership;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    Ok(match msg {
        QueryMsg::State {} => to_json_binary(&query_state(deps.storage)?),
        QueryMsg::Parameters {} => to_json_binary(&query_params(deps.storage)?),
        QueryMsg::Validators {} => to_json_binary(&query_validators(deps.storage)?),
        QueryMsg::StakingLiquidity {
            delegator,
            denom,
            validators,
        } => to_json_binary(&query_staking_liquidity(
            deps, env, delegator, denom, validators,
        )?),
        QueryMsg::RewardBalance {} => to_json_binary(&(query_reward_balance(deps.storage)?)),
        QueryMsg::UnbondRecord {
            staker,
            released,
            id,
            batch_id,
            min,
            max,
        } => to_json_binary(
            &(query_unbond_record(deps.storage, staker, released, id, batch_id, min, max)?),
        ),
        QueryMsg::QuoteToken { channel_id } => {
            to_json_binary(&query_quote_token(deps.storage, channel_id)?)
        }
        QueryMsg::Ownership {} => to_json_binary(&get_ownership(deps.storage)?),
        QueryMsg::Version {} => to_json_binary(&query_version(deps.storage)?),
        QueryMsg::Batch { status, min, max } => {
            to_json_binary(&query_batch(deps.storage, status, min, max)?)
        }
    }?)
}

pub fn query_quote_token(
    storage: &dyn Storage,
    channel_id: u32,
) -> Result<QuoteToken, ContractError> {
    let token = QUOTE_TOKEN.load(storage, channel_id)?;
    Ok(token)
}

pub fn query_version(storage: &dyn Storage) -> Result<ContractVersion, ContractError> {
    let ver = cw2::get_contract_version(storage)?;
    Ok(ver)
}

pub fn query_state(storage: &dyn Storage) -> Result<State, ContractError> {
    let state = STATE.load(storage)?;
    Ok(state)
}

pub fn query_params(storage: &dyn Storage) -> Result<Parameters, ContractError> {
    let params = PARAMETERS.load(storage)?;
    Ok(params)
}

pub fn query_validators(storage: &dyn Storage) -> Result<ValidatorsRegistry, ContractError> {
    let validators = VALIDATORS_REGISTRY.load(storage)?;
    Ok(validators)
}

pub fn query_reward_balance(storage: &dyn Storage) -> Result<Balance, ContractError> {
    let balance = REWARD_BALANCE.load(storage)?;
    Ok(Balance { amount: balance })
}

pub fn query_staking_liquidity(
    deps: Deps,
    env: Env,
    delegator: Option<String>,
    coin_denom: Option<String>,
    validators_list: Option<Vec<String>>,
) -> Result<StakingLiquidity, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let the_delegator = delegator
        .clone()
        .unwrap_or_else(|| env.contract.address.to_string());

    let denom = coin_denom
        .clone()
        .unwrap_or_else(|| params.underlying_coin_denom.to_string());

    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let validators_addr: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    let validators = validators_list.clone().unwrap_or_else(|| validators_addr);

    let delegated_amount = get_actual_total_delegated(
        deps.querier,
        the_delegator.to_string(),
        denom.clone(),
        validators.clone(),
    )?;

    let unclaimed_reward = get_unclaimed_reward(
        deps.querier,
        the_delegator.to_string(),
        denom.clone(),
        validators,
    )?;

    let reward_contract_balance = deps
        .querier
        .query_balance(params.reward_address.to_string(), denom)?;

    let total_reward = unclaimed_reward + reward_contract_balance.amount;
    let fee = calc::calc_with_rate(total_reward, params.fee_rate);
    let net_reward = total_reward - fee;

    let total_bond_amount = delegated_amount + net_reward;

    let state: State = STATE.load(deps.storage)?;
    let mut exchange_rate: Decimal = Decimal::one();
    if total_bond_amount != Uint128::zero() && state.total_supply != Uint128::zero() {
        exchange_rate = Decimal::from_ratio(total_bond_amount, state.total_supply);
    }

    Ok(StakingLiquidity {
        amount: total_bond_amount,
        delegated: delegated_amount,
        reward: total_reward,
        unclaimed_reward,
        exchange_rate,
        time: env.block.time,
        total_supply: state.total_supply,
    })
}

pub fn query_unbond_record(
    storage: &dyn Storage,
    staker: Option<String>,
    released: Option<bool>,
    id: Option<u64>,
    batch_id: Option<u64>,
    min: Option<u64>,
    max: Option<u64>,
) -> Result<Vec<UnbondRecord>, ContractError> {
    if id.is_some() {
        let unbonded_list = vec![unbond_record().load(storage, id.unwrap())?];
        return Ok(unbonded_list);
    }

    let (min_id, max_id) = calculate_query_bounds(min, max);
    let min_bound = Some(cw_storage_plus::Bound::Inclusive((min_id, PhantomData)));
    let max_bound = Some(cw_storage_plus::Bound::Inclusive((max_id, PhantomData)));

    let unbonded_range = if let Some(batch_id) = batch_id {
        unbond_record()
            .idx
            .batch
            .prefix(batch_id.to_string())
            .range(storage, min_bound, max_bound, Order::Ascending)
    } else {
        match (staker, released) {
            (Some(staker), None) => unbond_record().idx.staker.prefix(staker).range(
                storage,
                min_bound,
                max_bound,
                Order::Ascending,
            ),
            (None, Some(released)) => unbond_record()
                .idx
                .released
                .prefix(released.to_string())
                .range(storage, min_bound, max_bound, Order::Ascending),
            (Some(staker), Some(released)) => unbond_record()
                .idx
                .staker_released
                .prefix(format!("{}-{}", staker, released))
                .range(storage, min_bound, max_bound, Order::Ascending),
            (None, None) => return Err(ContractError::InvalidUnbondRecordQuery {}),
        }
    };

    Ok(unbonded_range
        .filter_map(|unbonded| Some(unbonded.ok()?.1))
        .collect())
}

pub fn query_unbond_record_from_batch(storage: &dyn Storage, batch_id: u64) -> Vec<UnbondRecord> {
    let mut unbonded_list: Vec<UnbondRecord> = vec![];
    let unbonded_range = unbond_record()
        .idx
        .released
        .prefix(batch_id.to_string())
        .range(storage, None, None, Order::Ascending);

    for unbonded in unbonded_range {
        if unbonded.is_ok() {
            unbonded_list.push(unbonded.unwrap().1);
        }
    }
    unbonded_list
}

pub fn query_batch(
    storage: &dyn Storage,
    status: Option<BatchStatus>,
    min: Option<u64>,
    max: Option<u64>,
) -> Result<Vec<Batch>, ContractError> {
    // if batch status parameter is none, set to pending as default
    let batch_status = match status {
        Some(status) => status,
        None => BatchStatus::Pending,
    };

    let min_bound = match min {
        Some(min) => Some(cw_storage_plus::Bound::Inclusive((min, PhantomData))),
        None => Some(cw_storage_plus::Bound::Inclusive((1, PhantomData))),
    };
    let max_bound = match max {
        Some(max) => {
            let max_id = if min.is_some() && max > min.unwrap() + 50 {
                min.unwrap() + 50
            } else {
                max
            };
            Some(cw_storage_plus::Bound::Inclusive((max_id, PhantomData)))
        }
        None => {
            let max_id = if min.is_some() { min.unwrap() + 50 } else { 50 };
            Some(cw_storage_plus::Bound::Inclusive((max_id, PhantomData)))
        }
    };

    let mut batch_list: Vec<Batch> = vec![];
    let batches = batches().idx.status.prefix(batch_status.to_string()).range(
        storage,
        min_bound,
        max_bound,
        Order::Ascending,
    );

    for batch in batches {
        if batch.is_ok() {
            batch_list.push(batch.unwrap().1);
        }
    }
    Ok(batch_list)
}
