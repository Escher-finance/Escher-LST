use std::collections::BTreeMap;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_TREASURY_FEE: Uint128 = Uint128::new(100_000);
/// The maximum allowed unbonding period is 42 days,
/// which is twice the typical staking period of a Cosmos SDK-based chain.
pub const MAX_UNBONDING_PERIOD: u64 = 3_628_800;

#[cw_serde]
pub struct BatchExpectedAmount {
    pub batch_id: u64,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Batch {
    /// Total amount of `stTIA` to be burned in this batch
    pub total_lst_to_burn: Uint128,

    // TODO: This should be a separate mapping storage
    pub liquid_unbond_requests: BTreeMap<String, LiquidUnbondRequest>,

    /// The length of the unstake requests list.
    ///
    /// Multiple unbond requests in a batch are aggregated into one unstake request per user.
    pub unstake_requests_count: u64,

    pub state: BatchState,
}

// Batch should always be constructed with a pending status
// Contract: Caller determines batch data
impl Batch {
    // TODO: BatchId type
    pub fn new_pending(submit_time: u64) -> Self {
        Self {
            total_lst_to_burn: 0_u128.into(),
            state: BatchState::Pending { submit_time },
            liquid_unbond_requests: Default::default(),
            unstake_requests_count: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub enum BatchState {
    /// Initial state of a batch. Only one batch is pending at a time (see [`crate::state::PENDING_BATCH_ID`].
    Pending {
        /// The earliest timestamp at which the batch can be submitted.
        ///
        /// This will be `creation_time + batch_period`.
        ///
        /// Note that this is a *minimum* timestamp - empty batches will not be submitted.
        submit_time: u64,
    },
    /// The batch has been submitted, and all unbonding requests have been processed. The unbonded tokens have not yet been sent back to this contract for withdrawing by the unbonded stakers.
    ///
    /// Unbonding requests can only be processed after the unbonding period of the chain this contract is running on.
    Submitted {
        /// Estimated time when the batch will be received.
        ///
        /// This will be `submission_time + unbonding_period`.
        receive_time: u64,
        /// The amount of native tokens that should be received after unstaking.
        expected_native_unstaked: Uint128,
    },
    /// The unbonding period has elapsed and the unbonded tokens have been sent back to this contract. The unbonded stakers from this batch are now able to claim their unbonded tokens.
    Received {
        /// The amount of native tokens received after unbonding.
        received_native_unstaked: Uint128,
    },
}

impl BatchState {
    pub const fn status(&self) -> BatchStatus {
        match self {
            BatchState::Pending { .. } => BatchStatus::Pending,
            BatchState::Submitted { .. } => BatchStatus::Submitted,
            BatchState::Received { .. } => BatchStatus::Received,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, JsonSchema)]
pub enum BatchStatus {
    /// See [`BatchState::Pending`].
    Pending,
    /// See [`BatchState::Submitted`].
    Submitted,
    /// See [`BatchState::Received`].
    Received,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct LiquidUnbondRequest {
    /// The user's address
    pub user: Addr,
    /// The user's share in the batch
    pub shares: Uint128,
    pub redeemed: bool,
}
