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

    #[error("slippage error: got {output_amount}, expected not smaller than {min_amount} or more than {max_amount}")]
    SlippageError {
        output_amount: cosmwasm_std::Uint128,
        min_amount: cosmwasm_std::Uint128,
        max_amount: cosmwasm_std::Uint128,
    },

    #[error("error when computing the instantiate2 address: {0}")]
    Instantiate2AddressError(#[from] cosmwasm_std::Instantiate2AddressError),

    #[error("not enough reward balance")]
    NotEnoughRewardBalance {},
}
