use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

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

    #[error("error when computing the instantiate2 address: {0}")]
    Instantiate2AddressError(#[from] cosmwasm_std::Instantiate2AddressError),
}
