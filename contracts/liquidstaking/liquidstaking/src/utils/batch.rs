use core::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};

#[cw_serde]
pub enum BatchStatus {
    Pending, // Pending means this batch still waiting the submit batch call after batch period to be submitted
    Submitted, // Submitted means this batch already processed and send undelegate message to validators
    Received, // received means this batch already received native token from validator undelegation as it already complete unbonding
    Released, // released means it already send back the unstaked native token to user and batch is completed/done
}

#[cw_serde]
pub struct Batch {
    /// ID of this batch
    pub id: u64,
    /// Total amount of `liquid staked token` to be burned in this batch
    pub total_liquid_stake: Uint128,
    /// The amount of native tokens that should be received after unbonding
    pub expected_native_unstaked: Option<Uint128>,
    /// The amount of native tokens received after unbonding
    pub received_native_unstaked: Option<Uint128>,

    pub unbond_records_count: u64,

    /// Estimated time when next batch action occurs
    pub next_batch_action_time: Option<u64>,

    pub status: BatchStatus,
}

// Batch should always be constructed with a pending status
// Contract: Caller determines batch data
impl Batch {
    pub fn new(id: u64, total_liquid_stake: Uint128, est_next_batch_action: u64) -> Self {
        Self {
            id,
            total_liquid_stake,
            next_batch_action_time: Some(est_next_batch_action),
            status: BatchStatus::Pending,
            expected_native_unstaked: None,
            received_native_unstaked: None,
            unbond_records_count: 0,
        }
    }
    pub fn update_status(&mut self, new_status: BatchStatus, next_action_time: Option<u64>) {
        match new_status {
            // next batch time =  env.block.time + batch period
            BatchStatus::Pending => {
                self.status = new_status;
                self.next_batch_action_time = next_action_time;
            }
            // next batch time = env.block.time + unbonding period
            BatchStatus::Submitted => {
                self.status = new_status;
                self.next_batch_action_time = next_action_time;
            }
            BatchStatus::Received => {
                self.status = new_status;
                self.next_batch_action_time = None;
            }
            BatchStatus::Released => {
                self.status = new_status;
                self.next_batch_action_time = None;
            }
        }
    }
}

impl fmt::Display for BatchStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BatchStatus::Pending => write!(f, "pending"),
            BatchStatus::Submitted => write!(f, "submitted"),
            BatchStatus::Received => write!(f, "received"),
            BatchStatus::Released => write!(f, "released"),
        }
    }
}

pub struct BatchIndexes<'a> {
    pub status: MultiIndex<'a, String, Batch, u64>,
}

impl<'a> IndexList<Batch> for BatchIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Batch>> + '_> {
        let v: Vec<&dyn Index<Batch>> = vec![&self.status];
        Box::new(v.into_iter())
    }
}

const BATCH_NAMESPACE: &str = "batch";

pub fn batches<'a>() -> IndexedMap<u64, Batch, BatchIndexes<'a>> {
    let indexes = BatchIndexes {
        status: MultiIndex::new(
            |_pk, d: &Batch| d.status.to_string(),
            BATCH_NAMESPACE,
            "batch__status",
        ),
    };
    IndexedMap::new(BATCH_NAMESPACE, indexes)
}
