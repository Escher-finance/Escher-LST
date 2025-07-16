use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("semver parse error: {0}")]
    SemverError(#[from] semver::Error),

    #[error("{0}")]
    Ownership(#[from] OwnershipError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("NoBalance")]
    NoBalance {},

    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("InvalidContractName")]
    InvalidContractName {},

    #[error("invalid migration version: expected {expected}, got {actual}")]
    InvalidMigrationVersion { expected: String, actual: String },

    #[error("this contract must have an owner")]
    OwnershipCannotBeRenounced,

    #[error("invalid fee rate")]
    InvalidFeeRate {},
}
