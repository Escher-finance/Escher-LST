use cosmwasm_std::StdError;
use cw20_base::ContractError as Cw20ContractError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Ownership(#[from] OwnershipError),

    #[error("{0}")]
    Cw20Error(#[from] Cw20ContractError),
}
