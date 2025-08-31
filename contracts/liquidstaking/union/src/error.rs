use cosmwasm_std::{Addr, StdError, Timestamp};
use cw2::VersionError;
use cw_utils::PaymentError;
use thiserror::Error;

use crate::types::{BatchId, BatchState, Staker, MAX_FEE_RATE};

pub type ContractResult<T> = core::result::Result<T, ContractError>;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error("unauthorized: {sender}")]
    Unauthorized { sender: Addr },

    #[error("no pending owner to claim")]
    NoPendingOwner,

    #[error("the caller is not the pending owner")]
    CallerIsNotPendingOwner,

    #[error("ownership transfer not ready")]
    OwnershipTransferNotReady { time_to_claim: Timestamp },

    #[error("Payment error: {0}")]
    Payment(#[from] PaymentError),

    #[error("Minimum liquid stake amount not met")]
    MinimumLiquidStakeAmount {
        minimum_stake_amount: u128,
        sent_amount: u128,
    },

    #[error("Unable to mint liquid staking token")]
    MintError,

    #[error("Validator already exists")]
    DuplicateValidator { validator: String },

    #[error("Validator not found")]
    ValidatorNotFound { validator: String },

    #[error("Address is not valid")]
    InvalidAddress,

    #[error("Batch is not ready to be submitted")]
    BatchNotReady { actual: u64, expected: u64 },

    #[error("Batch has already been submitted")]
    BatchAlreadySubmitted,

    #[error("No liquid unstake requests in batch")]
    BatchEmpty,

    #[error("batch not found")]
    BatchNotFound { batch_id: BatchId },

    #[error("the batch is still pending")]
    BatchStillPending,

    #[error("the batch has already been received")]
    BatchAlreadyReceived,

    #[error("Batch is either already closed or is in an error state")]
    BatchNotClaimable {
        batch_id: BatchId,
        status: BatchState,
    },

    #[error("the batch has not yet been submitted")]
    BatchNotYetSubmitted { batch_id: BatchId },

    #[error("Batch {batch_id} don't have the expected native amount")]
    BatchWithoutExpectedNativeAmount { batch_id: BatchId },

    #[error(
        "Received wrong batch amount, batch_id {batch_id} expected {expected}, got {received}"
    )]
    ReceivedWrongBatchAmount {
        batch_id: BatchId,
        expected: u128,
        received: u128,
    },

    #[error("the batch is not yet received")]
    BatchNotYetReceived,

    #[error("staker {staker} not found in batch (hash={})", staker.hash())]
    NoRequestInBatch { staker: Staker },

    #[error("Minimum liquid stake amount not met")]
    InvalidUnstakeAmount {
        total_liquid_stake_token: u128,
        amount_to_unstake: u128,
    },

    #[error("contract was intentionally stopped")]
    Stopped,

    #[error("contract is not stopped")]
    NotStopped,

    #[error("Receive rewards are smaller then the fee")]
    ReceiveRewardsTooSmall { amount: u128, minimum: u128 },

    #[error("The computed fees are zero for the received rewards: {received_rewards}")]
    ComputedFeesAreZero { received_rewards: u128 },

    #[error("No liquid stake to distribute rewards to")]
    NoLiquidStake,

    #[error("Calculated mint amount not as expected")]
    MintAmountMismatch { expected: u128, actual: u128 },

    #[error("{0}")]
    Version(#[from] VersionError),

    #[error("protocol fee rate can't be higher then {MAX_FEE_RATE}")]
    InvalidProtocolFeeRate,
    #[error(
        "the batch period ({batch_period}) is larger than the \
        queried unbonding period ({unbonding_period})"
    )]
    BatchPeriodLargerThanUnbondingPeriod {
        batch_period: u64,
        unbonding_period: u64,
    },
}
