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

    #[error("sender is not zkgm")]
    OnlyZkgm {},

    #[error("lane (channel-id {channel_id}) has not been configured to be fungible")]
    LaneNotFungible { channel_id: u32 },

    #[error("receiver is not valid")]
    InvalidReceiver {},

    #[error("only finalized txs are currently supported")]
    OnlyFinalized {},
}
