use cosmwasm_std::{StdError, Timestamp, Uint128};
use cw2::VersionError;
use cw_controllers::AdminError;
use cw_utils::PaymentError;
use crate::types::BatchState;
use thiserror::Error;

pub type ContractResult<T> = core::result::Result<T, ContractError>;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error("unauthorized: {sender}")]
    Unauthorized { sender: String },

    #[error("admin error: {0}")]
    Admin(#[from] AdminError),

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
        minimum_stake_amount: Uint128,
        sent_amount: Uint128,
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
    BatchNotFound { batch_id: u64 },

    #[error("the batch is still pending")]
    BatchStillPending,

    #[error("the batch has already been received")]
    BatchAlreadyReceived,

    #[error("Batch is either already closed or is in an error state")]
    BatchNotClaimable { batch_id: u64, status: BatchState },

    #[error("the batch has not yet been submitted")]
    BatchNotYetSubmitted { batch_id: u64 },

    #[error("Batch {batch_id} don't have the expected native amount")]
    BatchWithoutExpectedNativeAmount { batch_id: u64 },

    #[error(
        "Received wrong batch amount, batch_id {batch_id} expected {expected}, got {received}"
    )]
    ReceivedWrongBatchAmount {
        batch_id: u64,
        expected: Uint128,
        received: Uint128,
    },

    #[error("the batch is not yet received")]
    BatchNotYetReceived,

    #[error("staker '{staker}' not found in batch")]
    NoRequestInBatch { staker: String },

    #[error("Minimum liquid stake amount not met")]
    InvalidUnstakeAmount {
        total_liquid_stake_token: Uint128,
        amount_to_unstake: Uint128,
    },

    #[error("contract was intentionally stopped")]
    Stopped,

    #[error("contract is not stopped")]
    NotStopped,

    #[error("Config provided is wrong")]
    ConfigWrong,

    #[error("format error")]
    FormatError,

    #[error("Failed ibc transfer")]
    FailedIBCTransfer { msg: String },

    #[error("Contract already locked")]
    ContractLocked { msg: String },

    #[error("Receive rewards are smaller then the fee")]
    ReceiveRewardsTooSmall { amount: Uint128, minimum: Uint128 },

    #[error("The computed fees are zero for the received rewards: {received_rewards}")]
    ComputedFeesAreZero { received_rewards: Uint128 },

    #[error("No liquid stake to distribute rewards to")]
    NoLiquidStake,

    #[error("Calculated mint amount not as expected")]
    MintAmountMismatch { expected: Uint128, actual: Uint128 },

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error(
        "If liquid staking is done from a non native Osmosis address you need to provide an address via 'mint_to'"
    )]
    MissingMintAddress,

    #[error("The treasury address has not been configured")]
    TreasuryNotConfigured,

    #[error("{0}")]
    Version(#[from] VersionError),

    #[error("Can't recover packets with different receivers")]
    InvalidReceiver,

    #[error("Can't recover packets with different denoms")]
    InconsistentDenom,

    #[error("The contract is migrating to a newer version")]
    Migrating,

    #[error("DAO treasury fee can't be higher then 100000")]
    InvalidDaoTreasuryFee,
    #[error(
        "the batch period ({batch_period}) is larger than the \
        queried unbonding period ({unbonding_period})"
    )]
    BatchPeriodLargerThanUnbondingPeriod {
        batch_period: u64,
        unbonding_period: u64,
    },
}
