use cosmwasm_std::{Addr, Deps, Order, StdResult};
use depolama::StorageExt;
use itertools::Itertools;
use unionlabs_primitives::H256;

use crate::{
    helpers::get_rates,
    msg::{AccountingStateResponse, BatchesResponse, ConfigResponse},
    state::{
        AccountingStateStore, Batches, ConfigStore, LstAddress, Monitors, PendingBatchId,
        ProtocolFeeConfigStore, Stopped, UnstakeRequestsByStakerHash,
    },
    types::{Batch, BatchId, BatchStatus, Config, UnstakeRequest, UnstakeRequestKey},
};

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let Config {
        native_token_denom,
        minimum_liquid_stake_amount,
        batch_period_seconds,
    } = deps.storage.read_item::<ConfigStore>()?;

    Ok(ConfigResponse {
        native_token_denom,
        minimum_liquid_stake_amount: minimum_liquid_stake_amount.into(),
        protocol_fee_config: deps.storage.read_item::<ProtocolFeeConfigStore>()?,
        lst_address: deps.storage.read_item::<LstAddress>()?,
        monitors: deps
            .storage
            .read_item::<Monitors>()?
            .into_iter()
            .map(Addr::unchecked)
            .collect(),
        batch_period_seconds,
        stopped: deps.storage.read_item::<Stopped>()?,
    })
}

pub fn query_state(deps: Deps) -> StdResult<AccountingStateResponse> {
    let accounting_state = deps.storage.read_item::<AccountingStateStore>()?;

    let (redemption_rate, purchase_rate) = get_rates(&accounting_state);
    let res = AccountingStateResponse {
        total_bonded_native_tokens: accounting_state.total_bonded_native_tokens.into(),
        total_issued_lst: accounting_state.total_issued_lst.into(),
        total_reward_amount: accounting_state.total_reward_amount.into(),
        redemption_rate,
        purchase_rate,
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
            .iter()
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

pub fn query_unstake_requests_by_staker_hash(
    deps: Deps,
    staker_hash: H256,
) -> StdResult<Vec<UnstakeRequest>> {
    deps.storage
        .iter_range::<UnstakeRequestsByStakerHash>(
            Order::Ascending,
            UnstakeRequestKey {
                batch_id: BatchId::ONE,
                staker_hash,
            }..=UnstakeRequestKey {
                batch_id: BatchId::MAX,
                staker_hash,
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
