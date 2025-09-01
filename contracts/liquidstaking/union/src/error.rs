use cosmwasm_std::{Addr, StdError};
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

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error("unauthorized: {sender}")]
    Unauthorized { sender: Addr },

    #[error("no pending owner to claim")]
    NoPendingOwner,

    #[error("the caller is not the pending owner")]
    CallerIsNotPendingOwner,

    #[error("ownership transfer not ready, claimable at {time_to_claim_seconds}")]
    OwnershipTransferNotReady { time_to_claim_seconds: u64 },

    #[error(
        "attempted to bond less than minimum stake amount \
        (min={minimum_stake_amount}, sent={sent_amount})"
    )]
    MinimumLiquidStakeAmount {
        minimum_stake_amount: u128,
        sent_amount: u128,
    },

    #[error("computed mint amount is zero")]
    ComputedMintAmountIsZero,

    #[error("batch is not ready to be submitted (now={now}, ready_at={ready_at})")]
    BatchNotReady { now: u64, ready_at: u64 },

    #[error("batch {batch_id} has already been submitted")]
    BatchAlreadySubmitted { batch_id: BatchId },

    #[error("no liquid unstake requests in batch {batch_id}")]
    BatchEmpty { batch_id: BatchId },

    #[error("batch {batch_id} not found")]
    BatchNotFound { batch_id: BatchId },

    #[error("batch {batch_id} is still pending")]
    BatchStillPending { batch_id: BatchId },

    #[error("batch {batch_id} has already been received")]
    BatchAlreadyReceived { batch_id: BatchId },

    #[error(
        "received wrong batch amount, batch_id {batch_id} \
        expected {expected}, got {received}"
    )]
    ReceivedWrongBatchAmount {
        batch_id: BatchId,
        expected: u128,
        received: u128,
    },

    #[error("batch {batch_id} is not yet received")]
    BatchNotYetReceived { batch_id: BatchId },

    #[error("staker {staker} not found in batch {batch_id} (hash={})", staker.hash())]
    NoRequestInBatch { batch_id: BatchId, staker: Staker },

    #[error(
        "unbond slippage exceeded (total_issued_lst={total_issued_lst}, \
        amount_to_unstake={amount_to_unstake})"
    )]
    UnbondSlippageExceeded {
        total_issued_lst: u128,
        amount_to_unstake: u128,
    },

    #[error("contract was intentionally stopped")]
    Stopped,

    #[error("contract is not stopped")]
    NotStopped,

    #[error(
        "received rewards ({received_rewards}) are \
        less than the protocol fee ({protocol_fee})"
    )]
    RewardsReceivedLessThanProtocolFee {
        received_rewards: u128,
        protocol_fee: u128,
    },

    #[error("computed fees are zero for the received rewards ({received_rewards})")]
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

    #[error(
        "attempted to unbond more native tokens {unbond_amount} than \
        total bonded native tokens {total_bonded_native_tokens}"
    )]
    AttemptedToUnbondMoreThanBonded {
        unbond_amount: u128,
        total_bonded_native_tokens: u128,
    },
}
