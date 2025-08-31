use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;
use depolama::StorageExt;
use itertools::Itertools;
use unionlabs_primitives::{Bytes, H256};

use crate::{
    helpers::get_rates,
    msg::{BatchesResponse, ConfigResponse, StateResponse},
    state::{Batches, PendingBatchId, UnstakeRequests, UnstakeRequestsByStakerHash},
    types::{Batch, BatchId, BatchStatus, Staker, UnstakeRequest, UnstakeRequestKey},
};

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    let res = ConfigResponse {
        protocol_fee_config: config.protocol_fee_config,
        liquid_stake_token_denom: config.liquid_stake_token_address,
        monitors: config.monitors,
        batch_period: config.batch_period_seconds,
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

pub fn query_batch(deps: Deps, batch_id: BatchId) -> StdResult<Option<Batch>> {
    deps.storage.maybe_read::<Batches>(&batch_id)
}

pub fn query_batches(
    deps: Deps,
    start_after: Option<BatchId>,
    limit: Option<usize>,
    status: Option<BatchStatus>,
) -> StdResult<BatchesResponse> {
    Ok(BatchesResponse {
        batches: deps
            .storage
            .iter_range::<Batches>(Order::Ascending, start_after.unwrap_or(BatchId::ONE)..)
            .filter_ok(|(_, batch)| {
                status
                    .as_ref()
                    .map(|s| batch.state.status() == *s)
                    .unwrap_or(true)
            })
            .take(limit.unwrap_or(usize::MAX))
            .collect::<Result<_, _>>()?,
    })
}

pub fn query_batches_by_ids(deps: Deps, batch_ids: &[BatchId]) -> StdResult<BatchesResponse> {
    Ok(BatchesResponse {
        batches: batch_ids
            .into_iter()
            .map(|batch_id| {
                deps.storage
                    .read::<Batches>(batch_id)
                    .map(|batch| (*batch_id, batch))
            })
            .collect::<Result<_, _>>()?,
    })
}

pub fn query_pending_batch(deps: Deps) -> StdResult<Batch> {
    deps.storage
        .read::<Batches>(&deps.storage.read_item::<PendingBatchId>()?)
}

pub fn query_unstake_requests(deps: Deps, staker: Staker) -> StdResult<Vec<UnstakeRequest>> {
    deps.storage
        .iter_range::<UnstakeRequestsByStakerHash>(
            Order::Ascending,
            UnstakeRequestKey {
                batch_id: BatchId::ONE,
                staker_hash: staker.hash(),
            }..=UnstakeRequestKey {
                batch_id: BatchId::MAX,
                staker_hash: staker.hash(),
            },
        )
        .map_ok(|(_, unstake_request)| unstake_request)
        .collect()
}

pub fn query_all_unstake_requests(
    deps: Deps,
    start_after: Option<UnstakeRequestKey>,
    limit: Option<usize>,
) -> StdResult<Vec<UnstakeRequest>> {
    deps.storage
        .iter_range::<UnstakeRequestsByStakerHash>(
            Order::Ascending,
            start_after.unwrap_or(UnstakeRequestKey {
                batch_id: BatchId::ONE,
                staker_hash: H256::new([0x00; 32]),
            })..,
        )
        .take(limit.unwrap_or(usize::MAX))
        .map_ok(|(_, unstake_request)| unstake_request)
        .collect()
}
