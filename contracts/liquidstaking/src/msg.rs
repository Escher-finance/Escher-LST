use std::collections::BTreeMap;

use crate::state::{Balance, Parameters, State, UnbondRecord, Validator, ValidatorsRegistry};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct InstantiateMsg {
    pub underlying_coin_denom: String,
    pub validators: Vec<Validator>,
    pub liquidstaking_denom: String,
    pub ucs01_channel: String,
    pub ucs01_relay_contract: String,
    pub revenue_receiver: String,
    pub unbonding_time: u64,
    pub cw20_address: Option<Addr>,
}

#[cw_serde]
pub enum ExecuteRewardMsg {
    SplitReward {},
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Delegate native denom `amount` to validator
    /// Issue `amount` / exchange_rate for the user.
    Bond {
        staker: Option<String>,
        amount: Option<Coin>,
    },
    /// Send liquid staking denom then undelegate native denom according exchange rate from validator
    Unbond {
        staker: Option<String>,
        amount: Option<Uint128>,
    },
    ProcessRewards {},
    Transfer {
        amount: Coin,
        receiver: Addr,
    },
    ProcessUnbonding {
        id: u64,
    },
    /// Set new token factory denom admin
    SetTokenAdmin {
        denom: String,
        new_admin: Addr,
    },
    /// Change parameters, only owner can do this
    SetParameters {
        underlying_coin_denom: Option<String>,
        liquidstaking_denom: Option<String>,
        ucs01_channel: Option<String>,
        ucs01_relay_contract: Option<String>,
        unbonding_time: Option<u64>,
        cw20_address: Option<Addr>,
        reward_address: Option<Addr>,
    },
    /// Update Validators
    UpdateValidators {
        validators: Vec<Validator>,
    },
    /// Reset will set state to initial state and unbond all delegations
    Reset {},
    /// Redelegate will delegate the balance
    Redelegate {},
}

#[cw_ownable_query]
#[non_exhaustive]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(State)]
    State {},
    #[returns(Parameters)]
    Parameters {},
    #[returns(ValidatorsRegistry)]
    Validators {},
    #[returns(StakingLiquidity)]
    StakingLiquidity {
        delegator: Option<String>,
        denom: Option<String>,
        validators: Option<Vec<String>>,
    },
    #[returns(Balance)]
    Balance {},
    #[returns(Log)]
    Log {},
    #[returns(Vec<UnbondRecord>)]
    UnbondRecord {
        staker: Option<String>,
        sender: Option<String>,
        released: Option<bool>,
    },
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
pub struct StakingLiquidity {
    pub amount: Uint128,
    pub delegated: Uint128,
    pub reward: Uint128,
    pub exchange_rate: Decimal,
    pub time: Timestamp,
}

#[cw_serde]
pub struct Log {
    pub message: String,
}

#[cw_serde]
pub struct MintTokensPayload {
    pub sender: String,
    pub staker: String,
    pub amount: Uint128,
}

#[cw_serde]
pub struct BondRewardsPayload {
    pub amount: Uint128,
    pub validator: String,
}

#[cw_serde]
pub struct UndelegationRecord {
    pub amount: Uint128,
    pub validator: Validator,
}

#[cw_serde]
pub enum DelegationDiff {
    Surplus,
    Deficit,
}

#[cw_serde]
pub struct ValidatorDelegation {
    pub address: String,
    pub delegation_diff: DelegationDiff,
    pub diff_amount: Uint128,
}
