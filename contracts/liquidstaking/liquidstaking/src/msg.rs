use std::collections::BTreeMap;

use crate::state::{Balance, Parameters, State, UnbondRecord, Validator, ValidatorsRegistry};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint128, Uint256};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use schemars::JsonSchema;
use unionlabs_primitives::{Bytes, H256};

#[cw_serde]
pub struct InstantiateMsg {
    pub underlying_coin_denom: String,
    pub validators: Vec<Validator>,
    pub liquidstaking_denom: String,
    pub ucs03_channel: String,
    pub ucs03_relay_contract: String,
    pub revenue_receiver: Addr,
    pub unbonding_time: u64,
    pub reward_code_id: u64,
    pub fee_rate: Decimal,
    pub cw20_address: Option<Addr>,
    pub salt: String,
}

#[cw_serde]
pub struct InstantiateRewardMsg {
    pub lst_contract: Addr,
    pub revenue_receiver: Addr,
    pub fee_rate: Decimal,
    pub coin_denom: String,
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
        salt: String,
    },
    /// Send liquid staking denom then undelegate native denom according exchange rate from validator
    Unbond {
        staker: Option<String>,
        amount: Option<Uint128>,
    },
    ProcessRewards {},
    ProcessUnbonding {
        id: u64,
        salt: String,
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
        ucs03_channel: Option<String>,
        ucs03_relay_contract: Option<String>,
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
    /// Move native balance to reward contract
    MoveToReward {},
    Transfer {
        amount: Coin,
        receiver: String,
        ucs03_channel_id: u32,
        ucs03_relay_contract: String,
        quote_token: String,
        salt: String,
    },
    TransferToOwner {},
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

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ucs03RelayExecuteMsg {
    /// This allows us to transfer via ucs03 relayer
    Transfer {
        channel_id: u32,
        receiver: Bytes,
        base_token: String,
        base_amount: Uint128,
        quote_token: Bytes,
        quote_amount: Uint256,
        timeout_height: u64,
        timeout_timestamp: u64,
        salt: H256,
    },
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
    pub salt: String,
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
