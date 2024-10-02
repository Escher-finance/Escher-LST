use std::collections::BTreeMap;

use crate::state::{Balance, Parameters, State, ValidatorsRegistry};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub underlying_coin_denom: String,
    pub validators: Vec<Addr>,
    pub liquidstaking_denom: String,
    pub ucs01_channel: String,
    pub ucs01_relay_contract: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    ////////////////////
    /// Owner's operations
    ////////////////////
    // UpdateConfig {
    //     owner: Option<String>,
    //     validators: Option<String>,
    // },

    ////////////////////
    /// User's operations
    ////////////////////

    /// Receives `amount` in underlying coin denom from sender.
    /// Delegate `amount` to validator
    /// Issue `amount` / exchange_rate for the user.
    Bond {
        source: String,
    },
    // BondRewards {},
    // Send back unbonded coin to the user
    // WithdrawUnbonded {},
    Transfer {
        amount: Coin,
        receiver: Addr,
    },
    SetOwner {
        new_owner: Addr,
    },
    SetTokenAdmin {
        denom: String,
        new_admin: Addr,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(State)]
    State {},
    #[returns(Parameters)]
    Parameters {},
    #[returns(ValidatorsRegistry)]
    Validators {},
    #[returns(TotalBond)]
    TotalBondAmount {
        delegator: String,
        denom: String,
        validators: Vec<String>,
    },
    #[returns(Balance)]
    Balance {},
    #[returns(Log)]
    Log {},
}

pub type Fees = BTreeMap<String, Coin>;

/// This is the message we accept via Receive
#[cw_serde]
pub struct TransferMsg {
    /// The local channel to send the packets on
    pub channel: String,
    /// The remote address to send to.
    pub receiver: String,
    /// How long the packet lives in seconds. If not specified, use default_timeout
    pub timeout: Option<u64>,
    /// The memo
    pub memo: String,
}

#[cw_serde]
pub enum Ucs01RelayExecuteMsg {
    /// This allows us to transfer native tokens
    Transfer(TransferMsg),
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct TotalBond {
    pub amount: Uint128,
    pub delegated: Uint128,
    pub reward: Uint128,
}

#[cw_serde]
pub struct Log {
    pub message: String,
}

#[cw_serde]
pub struct MintTokensPayload {
    pub source: String,
    pub amount: Uint128,
}
