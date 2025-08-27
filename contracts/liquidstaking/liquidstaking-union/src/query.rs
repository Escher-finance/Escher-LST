use std::marker::PhantomData;

use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Decimal, Deps, Env, Order, Storage, Uint128,
};
use cw2::ContractVersion;
use cw_ownable::get_ownership;

use crate::{
    msg::{QueryMsg, StakingLiquidity},
    state::{
        unbond_record, Balance, Parameters, QuoteToken, State, Status, UnbondRecord,
        ValidatorsRegistry, PARAMETERS, QUOTE_TOKEN, REWARD_BALANCE, STATE, STATUS,
        VALIDATORS_REGISTRY,
    },
    utils::{
        batch::{batches, Batch, BatchStatus},
        calc,
        calc::calculate_query_bounds,
        delegation::{get_actual_total_delegated, get_unclaimed_reward},
    },
    ContractError,
};

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
        QueryMsg::Batch {
            id,
            status,
            min,
            max,
        } => to_json_binary(&query_batch(deps.storage, id, status, min, max)?),
        QueryMsg::Status {} => to_json_binary(&query_status(deps.storage)?),
    }?)
}

pub fn query_status(storage: &dyn Storage) -> Result<Status, ContractError> {
    Ok(STATUS.load(storage)?)
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

    let validators = validators_list.clone().unwrap_or(validators_addr);

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

pub fn query_unreleased_unbond_record_from_batch(
    storage: &dyn Storage,
    batch_id: u64,
    limit: u32,
) -> Vec<UnbondRecord> {
    let mut unbonded_list: Vec<UnbondRecord> = vec![];
    let unbonded_range = unbond_record()
        .idx
        .batch
        .prefix(batch_id.to_string())
        .range(storage, None, None, Order::Ascending);

    let mut count = 0;
    for unbonded in unbonded_range {
        if unbonded.is_ok() {
            let unbond_record = unbonded.unwrap().1;
            if !unbond_record.released {
                unbonded_list.push(unbond_record);
                count += 1;
                if count >= limit {
                    break;
                }
            }
        }
    }
    unbonded_list
}

pub fn query_batch(
    storage: &dyn Storage,
    id: Option<u64>,
    status: Option<BatchStatus>,
    min: Option<u64>,
    max: Option<u64>,
) -> Result<Vec<Batch>, ContractError> {
    // if batch id parameter is provided, return the batch with that id
    if id.is_some() {
        let batch = batches().load(storage, id.unwrap())?;
        return Ok(vec![batch]);
    }

    // if batch status parameter is none, set to pending as default
    let batch_status = match status {
        Some(status) => status,
        None => BatchStatus::Pending,
    };

    let (min_id, max_id) = calculate_query_bounds(min, max);
    let min_bound = Some(cw_storage_plus::Bound::Inclusive((min_id, PhantomData)));
    let max_bound = Some(cw_storage_plus::Bound::Inclusive((max_id, PhantomData)));

    let mut batch_list: Vec<Batch> = vec![];
    let batches = batches().idx.status.prefix(batch_status.to_string()).range(
        storage,
        min_bound,
        max_bound,
        Order::Ascending,
    );

    #[allow(clippy::manual_flatten)]
    // https://github.com/Escher-finance/cw-liquid-staking/issues/145
    for batch in batches {
        if let Ok((_, batch)) = batch {
            batch_list.push(batch);
        }
    }
    Ok(batch_list)
}
