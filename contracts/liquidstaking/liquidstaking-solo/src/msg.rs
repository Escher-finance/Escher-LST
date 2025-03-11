use std::collections::BTreeMap;

use crate::state::{Parameters, QuoteToken, State, UnbondRecord, Validator, ValidatorsRegistry};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint128, Uint256};
use cw2::ContractVersion;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use schemars::JsonSchema;
use unionlabs_primitives::{Bytes, H256};

#[cw_serde]
pub struct InstantiateMsg {
    /// native coin denom
    pub underlying_coin_denom: String,
    /// list of validator address with weight
    pub validators: Vec<Validator>,
    /// liquid staking denom name
    pub liquidstaking_denom: String,
    /// ucs03 relay contract address
    pub ucs03_relay_contract: String,
    /// fee/revenue receiver address
    pub fee_receiver: Addr,
    /// unbonding time wait period in seconds
    pub unbonding_time: u64,
    /// reward contract code id
    pub reward_code_id: u64,
    /// fee/revenue rate from reward
    pub fee_rate: Decimal,
    /// cw20 liquid staking denom contract address
    pub cw20_address: Addr,
    /// salt that is used for ucs03 relayer transfer call
    pub salt: String,
    // tokens
    pub quote_tokens: Vec<QuoteToken>,
}

#[cw_serde]
pub struct InstantiateRewardMsg {
    pub lst_contract: Addr,
    pub fee_receiver: Addr,
    pub fee_rate: Decimal,
    pub coin_denom: String,
}

#[cw_serde]
pub enum ExecuteRewardMsg {
    MigrateMsg {},
    SplitReward {},
    SetConfig {
        fee_receiver: Option<Addr>,
        fee_rate: Option<Decimal>,
    },
    TransferToOwner {},
}

#[cw_ownable_execute]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Delegate native denom `amount` to validator
    /// Issue `amount` / exchange_rate for the user.
    Bond {
        amount: Option<Uint128>,
        salt: String,
    },
    /// Send liquid staking denom then undelegate native denom according exchange rate from validator
    Unbond {
        amount: Option<Uint128>,
    },
    // Withdraw staking rewards and call split reward to reward contract
    ProcessRewards {},
    // Process finished unbonding and send native token back to user
    ProcessUnbonding {
        id: u64,
        salt: String,
    },
    /// Change parameters, only owner can do this
    SetParameters {
        underlying_coin_denom: Option<String>,
        liquidstaking_denom: Option<String>,
        ucs03_relay_contract: Option<String>,
        unbonding_time: Option<u64>,
        cw20_address: Option<Addr>,
        reward_address: Option<Addr>,
        fee_receiver: Option<Addr>,
        fee_rate: Option<Decimal>,
    },
    /// Update quote token
    UpdateQuoteToken {
        channel_id: u32,
        quote_token: QuoteToken,
    },
    /// Update Validators
    UpdateValidators {
        validators: Vec<Validator>,
    },
    OnZkgm {
        channel_id: u32,
        sender: Bytes,
        message: Bytes,
    },
    /// Redelegate some amount that is called from reward contract as result of split reward call to reward contract
    Redelegate {},
    /// Call migrate to reward contract
    MigrateReward {
        code_id: u64,
    },
    /// Below are Utilities for development purpose only
    /// Move native balance to reward contract (for development phase only)
    MoveToReward {},
    /// Transfer utility (for development phase only)
    Transfer {
        amount: Uint128,
        base_denom: String,
        receiver: String,
        ucs03_channel_id: u32,
        ucs03_relay_contract: String,
        quote_token: String,
        salt: String,
    },
    NormalizeSupply {},
    /// Reset will set state to initial state and unbond all delegations (for development phase only)
    Reset {},
    /// Transfer all native balance of this contract to owner (for development purpose only)
    TransferToOwner {},
    // Utilities to transfer reward to this contract (for development only)
    TransferReward {},

    SetConfig {
        lst_contract_address: Option<Addr>,
        fee_receiver: Option<Addr>,
        fee_rate: Option<Decimal>,
        coin_denom: Option<String>,
    },
    Burn {
        amount: Uint128,
    },
}

#[cw_serde]
pub struct Balance {
    pub amount: Uint128,
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
        released: Option<bool>,
        id: Option<u64>,
        min: Option<u64>,
        max: Option<u64>,
    },
    #[returns(ContractVersion)]
    Version {},
    #[returns(QuoteToken)]
    QuoteToken { channel_id: u32 },
}

pub type Fees = BTreeMap<String, Coin>;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ucs03ExecuteMsg {
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
pub struct StakingLiquidity {
    pub amount: Uint128,
    pub delegated: Uint128,
    pub reward: Uint128,
    pub unclaimed_reward: Uint128,
    pub exchange_rate: Decimal,
    pub time: Timestamp,
    pub total_supply: Uint128,
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
    pub channel_id: Option<u32>,
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

#[cw_serde]
pub enum ZkgmMessage {
    Bond {
        amount: Uint128,
        salt: String,
        slippage: Option<Decimal>,
        expected: Uint128,
    },
    Unbond {
        amount: Uint128,
        slippage: Option<Decimal>,
        expected: Uint128,
    },
}

#[cw_serde]
pub struct BondData {
    pub mint_amount: Uint128,
    pub delegated_amount: Uint128,
    pub total_bond_amount: Uint128,
    pub exchange_rate: Decimal,
    pub total_supply: Uint128,
}

#[cw_serde]
pub struct UnbondData {
    pub undelegate_amount: Uint128,
    pub delegated_amount: Uint128,
    pub reward: Uint128,
    pub exchange_rate: Decimal,
    pub total_supply: Uint128,
    pub record_id: u64,
}

#[cw_serde]
pub struct MigrateMsg {}
