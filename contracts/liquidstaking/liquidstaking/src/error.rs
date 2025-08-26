use cosmwasm_std::{StdError, Uint128};
use cw_ownable::OwnershipError;
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

    #[error("Unauthorized")]
    Unauthorized {},

    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("InvalidAsset")]
    InvalidAsset {},

    #[error("NoAsset")]
    NoAsset {},

    #[error("NotEnoughFund")]
    NotEnoughFund {},

    #[error("requires at least one validator")]
    EmptyValidator {},

    #[error("NotEnoughAvailableFund")]
    NotEnoughAvailableFund {},

    #[error("ReplyError")]
    ReplyError { message: String },

    #[error("InvalidContractName")]
    InvalidContractName {},

    #[error("invalid migration version: expected {expected}, got {actual}")]
    InvalidMigrationVersion { expected: String, actual: String },

    #[error("ZeroSupplyOrDelegatedAmount")]
    ZeroSupplyOrDelegatedAmount {},

    #[error("this contract must have an owner")]
    OwnershipCannotBeRenounced,

    #[error("withdraw contract address is not set")]
    NoRewardAddress,

    #[error("InvalidCodeID")]
    InvalidCodeID { message: String },

    #[error("CompletedUnbondRecord")]
    CompletedUnbondRecord {},

    #[error("InvalidMintAmount")]
    InvalidMintAmount {},

    #[error("slippage error: got {output_amount}, expected not smaller than {min_amount} or more than {max_amount}")]
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

    #[error("invalid payload")]
    InvalidPayload {},

    #[error("cannot migrate reward contract that is equal with current contract")]
    InvalidRewardContractMigration {},

    #[error("bond amount is less than minimum bond amount, {actual} < {expected}")]
    BondAmountTooLow { actual: Uint128, expected: Uint128 },

    #[error("unbond amount is less than minimum unbond amount, {actual} < {expected}")]
    UnbondAmountTooLow { actual: Uint128, expected: Uint128 },

    #[error("This functionality is currently disabled for maintenance")]
    FunctionalityUnderMaintenance {},

    #[error("invalid fee rate")]
    InvalidFeeRate {},

    #[error("Invalid exchange rate")]
    InvalidExchangeRate {},

    #[error("error encode any msg")]
    EncodeAnyMsgError {},

    #[error("invalid channel id")]
    InvalidChannelId {},

    #[error("invalid {kind} address: {address}, reason: {reason}")]
    InvalidAddress {
        kind: String,
        address: String,
        reason: String,
    },

    #[error("no reward to normalize: {msg}")]
    NoRewardToNormalize { msg: String },

    #[error("Unauthorized Zkgm message, only hub contract is allowed")]
    ZkgmNotAuthorized {},
}
