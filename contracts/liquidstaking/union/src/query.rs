use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;
use itertools::Itertools;

use crate::{
    helpers::get_rates,
    msg::{BatchesResponse, ConfigResponse, StateResponse},
    state::{unstake_requests, UnstakeRequest, BATCHES, CONFIG, PENDING_BATCH_ID, STATE},
    types::{Batch, BatchStatus},
};

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    let res = ConfigResponse {
        protocol_fee_config: config.protocol_fee_config,
        liquid_stake_token_denom: config.liquid_stake_token_address,
        monitors: config.monitors,
        batch_period: config.batch_period,
        stopped: config.stopped,
    };
    Ok(res)
}

pub fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    let (_, purchase_rate) = get_rates(&state);
    let res = StateResponse {
        total_bonded_native_tokens: state.total_bonded_native_tokens,
        total_liquid_stake_token: state.total_issued_lst,
        rate: purchase_rate,
        pending_owner: state
            .pending_owner
            .map(|v| v.to_string())
            .unwrap_or_default(),
        total_reward_amount: state.total_reward_amount,
    };
    Ok(res)
}

pub fn query_batch(deps: Deps, id: u64) -> StdResult<Batch> {
    BATCHES.load(deps.storage, id)
}

pub fn query_batches(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<usize>,
    status: Option<BatchStatus>,
) -> StdResult<BatchesResponse> {
    Ok(BatchesResponse {
        batches: BATCHES
            .range(
                deps.storage,
                start_after.map(Bound::exclusive),
                None,
                Order::Ascending,
            )
            .filter_ok(|(_, v)| status.map(|s| v.state.status() == s).unwrap_or(true))
            .take(limit.unwrap_or(usize::MAX))
            .collect::<Result<_, _>>()?,
    })
}

pub fn query_batches_by_ids(deps: Deps, ids: Vec<u64>) -> StdResult<BatchesResponse> {
    Ok(BatchesResponse {
        batches: ids
            .into_iter()
            .map(|id| (id, BATCHES.load(deps.storage, id)))
            .map(|(id, r)| r.map(|b| (id, b)))
            .collect::<Result<_, _>>()?,
    })
}

pub fn query_pending_batch(deps: Deps) -> StdResult<Batch> {
    BATCHES.load(deps.storage, PENDING_BATCH_ID.load(deps.storage)?)
}

pub fn query_unstake_requests(deps: Deps, user: String) -> StdResult<Vec<UnstakeRequest>> {
    unstake_requests()
        .idx
        .by_user
        .prefix(user.to_string())
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|r| r.map(|(_, r)| r))
        .collect()
}

pub fn query_all_unstake_requests(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<UnstakeRequest>> {
    unstake_requests()
        .idx
        .by_user
        .range(
            deps.storage,
            start_after.map(|s| Bound::exclusive(("".to_string(), s))),
            None,
            cosmwasm_std::Order::Ascending,
        )
        .take(limit.unwrap_or(u32::MAX) as usize)
        .map(|r| r.map(|(_, r)| r))
        .collect::<Result<_, _>>()
}
