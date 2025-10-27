use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use cw_utils::PaymentError;
use thiserror::Error;

use crate::utils::batch::BatchStatus;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Ownership(#[from] OwnershipError),

    #[error("semver parse error: {0}")]
    SemverError(#[from] semver::Error),

    #[error("unauthorized")]
    Unauthorized {},

    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("invalid assset")]
    InvalidAsset {},

    #[error("no asset")]
    NoAsset {},

    #[error("not enough fund")]
    NotEnoughFund {},

    #[error("requires at least one validator")]
    EmptyValidator {},

    #[error("not enough available fund")]
    NotEnoughAvailableFund {},

    #[error("reply error: {message}")]
    ReplyError { message: String },

    #[error("invalid contract name")]
    InvalidContractName {},

    #[error("invalid migration version: expected {expected}, got {actual}")]
    InvalidMigrationVersion { expected: String, actual: String },

    #[error("zero supply or delegated amount")]
    ZeroSupplyOrDelegatedAmount {},

    #[error("this contract must have an owner")]
    OwnershipCannotBeRenounced,

    #[error("withdraw contract address is not set")]
    NoRewardAddress,

    #[error("invalid code id: {message}")]
    InvalidCodeID { message: String },

    #[error("completed unbond record")]
    CompletedUnbondRecord {},

    #[error("invalid mint amount")]
    InvalidMintAmount {},

    #[error(
        "slippage error: got {output_amount}, expected not smaller than {min_amount} or more than {max_amount}"
    )]
    SlippageError {
        output_amount: cosmwasm_std::Uint128,
        min_amount: cosmwasm_std::Uint128,
        max_amount: cosmwasm_std::Uint128,
    },

    #[error("error when computing the instantiate2 address: {0}")]
    Instantiate2AddressError(#[from] cosmwasm_std::Instantiate2AddressError),

    #[error("Unbond record query needs at least staker or released")]
    InvalidUnbondRecordQuery {},

    #[error("Validators must be unique by address and have non-zero weight")]
    InvalidValidators {},

    #[error("Quote tokens must be unique by channel_id")]
    InvalidQuoteTokens {},

    #[error("not enough reward balance")]
    NotEnoughRewardBalance {},

    #[error("no pending batch is available")]
    EmptyBatch {},

    #[error("bath status is incorrect, actual: {actual}, expected: {expected}")]
    BatchStatusIncorrect {
        actual: BatchStatus,
        expected: BatchStatus,
    },

    #[error("batch is not ready to be executed, actual: {actual}, expected: {expected}")]
    BatchNotReady { actual: u64, expected: u64 },

    #[error("batch unbonding not yet complete")]
    BatchIncompleteUnbonding {},

    #[error("batch received amount can not bigger than expected native unstaked amount")]
    InvalidBatchReceivedAmount {},

    #[error("InvalidContractName")]
    InvalidPayload {},

    #[error("cannot migrate reward contract that is equal with current contract")]
    InvalidRewardContractMigration {},

    #[error("bond amount is less than minimum bond amount")]
    BondAmountTooLow {},

    #[error("unbond amount is less than minimum unbond amount")]
    UnbondAmountTooLow {},

    #[error("This functionality is currently disabled for maintenance")]
    FunctionalityUnderMaintenance {},

    #[error("invalid fee rate")]
    InvalidFeeRate {},

    #[error("Invalid exchange rate")]
    InvalidExchangeRate {},

    #[error("error encode any msg")]
    EncodeAnyMsgError {},

    #[error("invalid recipient channel id")]
    InvalidChannelId {},

    #[error("invalid/unsupported recipient ibc channel id")]
    InvalidIBCChannelId {},

    #[error("invalid {kind} address/hex: {address} because: {reason}")]
    InvalidAddress {
        kind: String,
        address: String,
        reason: String,
    },

    #[error("no reward to normalize: {msg}")]
    NoRewardToNormalize { msg: String },

    #[error("recipient ibc channel id not found for unbond record with id: {id}")]
    UnbondRecordIbcChannelNotFound { id: u64 },

    #[error("batch next action time is not set")]
    BatchNextActionTimeNotSet,

    #[error("batch expected native unstaked is not set")]
    BatchExpectedNativeUnstakedNotSet,

    #[error("invalid undelegation unstake return native amount")]
    InvalidUnstakeReturnNativeAmount,

    #[error("no salt")]
    NoSalt,

    #[error("batch received amount: {received_amount} exceed {expected_native_unstaked}")]
    SlashBatchReceivedAmountExceedExpected {
        batch_id: u64,
        received_amount: cosmwasm_std::Uint128,
        expected_native_unstaked: cosmwasm_std::Uint128,
    },

    #[error("unsupported operation: {msg}")]
    UnsupportedOperation { msg: String },

    #[error("insufficient allowance, {allowance} is lower than {required}")]
    InsufficientAllowance {
        allowance: cosmwasm_std::Uint128,
        required: cosmwasm_std::Uint128,
    },

    #[error(transparent)]
    Payment(#[from] PaymentError),
}
