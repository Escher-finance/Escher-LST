use std::collections::BTreeMap;

use cosmwasm_schema::{QueryResponses, cw_serde};
use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint64, Uint128, Uint256};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use cw2::ContractVersion;
use cw20::Cw20ReceiveMsg;
use serde::{Deserialize, Serialize};
use unionlabs_primitives::{Bytes, H256};

use crate::{
    state::{
        Balance, Parameters, QuoteToken, State, Status, UnbondRecord, Validator, ValidatorsRegistry,
    },
    types::ChannelId,
    utils::batch::{Batch, BatchStatus},
};

#[cw_serde]
pub struct InstantiateMsg {
    /// native coin denom like ubbn, emuno, etc.
    pub underlying_coin_denom: String,
    /// native coin denom symbol that will be used for ucs03 transfer like ubbn, emuno, etc.
    pub underlying_coin_denom_symbol: String,
    /// liquid staking cw20 denom name that will be used for ucs03 transfer like ebbn, emuno, etc.
    pub liquidstaking_denom: String,
    /// liquid staking cw20 denom symbol that will be used for ucs03 transfer like eBABY
    pub liquidstaking_denom_symbol: String,
    /// list of validator address with weight
    pub validators: Vec<Validator>,
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
    // batch period range in seconds to execute batch
    pub batch_period: u64,
    // minimum bond/stake amount
    pub min_bond: Uint128,
    // minimum unbond/unstake amount
    pub min_unbond: Uint128,
    // limit per batch
    // this is the max number of unbonding records that can be processed in one batch
    pub batch_limit: u32,
    // handler of cw20 staking token transfer, as ucs03 fee payer address and also minted cw20 staking token receiver
    pub transfer_handler: String,
    // ucs03 transfer fee from babylon to other
    pub transfer_fee: Uint128,
    // zkgm token_minter address as cw20 allowance spender
    pub zkgm_token_minter: String,
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
    SplitReward {},
    SetConfig {
        fee_receiver: Option<Addr>,
        fee_rate: Option<Decimal>,
        lst_contract_address: Option<Addr>,
        coin_denom: Option<String>,
    },
}

#[cw_serde]
pub enum Cw20PayloadMsg {
    Unstake {
        recipient: Option<String>,
        recipient_channel_id: Option<u32>,
    },
}

#[cw_ownable_execute]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Delegate native denom `amount` to validator
    /// Issue `amount` / exchange_rate for the user.
    Bond {
        slippage: Option<Decimal>,
        expected: Uint128,
        recipient: Option<String>,
        recipient_channel_id: Option<u32>,
        salt: Option<String>,
    },
    /// Receive liquid staking cw20 token denom then undelegate native denom according exchange rate from validator
    Receive(Cw20ReceiveMsg),
    /// Submit pending batch
    SubmitBatch {},
    // Withdraw staking rewards and call split reward to reward contract
    ProcessRewards {},
    SetBatchReceivedAmount {
        id: u64,
        amount: Uint128,
    },
    // Process batch with complete unbonding(already receive token) to automatic withdraw and send native token back to user
    ProcessBatchWithdrawal {
        id: u64,
        salt: Vec<String>,
    },
    /// Change parameters, only owner can do this
    SetParameters {
        underlying_coin_denom: Option<String>,
        liquidstaking_denom: Option<String>,
        underlying_coin_denom_symbol: Option<String>,
        liquidstaking_denom_symbol: Option<String>,
        ucs03_relay_contract: Option<String>,
        unbonding_time: Option<u64>,
        cw20_address: Option<Addr>,
        reward_address: Option<Addr>,
        fee_receiver: Option<Addr>,
        fee_rate: Option<Decimal>,
        batch_period: Option<u64>,
        min_bond: Option<Uint128>,
        min_unbond: Option<Uint128>,
        batch_limit: Option<u32>,
        transfer_handler: Option<String>,
        transfer_fee: Option<Uint128>,
        zkgm_token_minter: Option<String>,
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
    // Zkgm protocal handler
    OnZkgm {
        caller: Addr,
        path: Uint256,
        source_channel_id: ChannelId,
        destination_channel_id: ChannelId,
        sender: Bytes,
        message: Bytes,
        relayer: Addr,
        relayer_msg: Bytes,
    },
    /// Redelegate some amount that is called from reward contract as result of split reward call to reward contract
    Redelegate {},
    /// Call migrate to reward contract
    MigrateReward {
        code_id: u64,
    },
    // Set maintenance status
    SetStatus(Status),
    // Set supported ucs03 chain
    SetChain {
        chain: crate::state::Chain,
    },
    // Remove ucs03 chain
    RemoveChain {
        channel_id: u32,
    },
    // Inject by staking without minting liquid staking token
    Inject {
        amount: Uint128,
    },
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
    RewardBalance {},
    #[returns(Vec<UnbondRecord>)]
    UnbondRecord {
        staker: Option<String>,
        released: Option<bool>,
        id: Option<u64>,
        batch_id: Option<u64>,
        min: Option<u64>,
        max: Option<u64>,
    },
    #[returns(ContractVersion)]
    Version {},
    #[returns(QuoteToken)]
    QuoteToken { channel_id: u32 },
    #[returns(Batch)]
    Batch {
        id: Option<u64>,
        status: Option<BatchStatus>,
        min: Option<u64>,
        max: Option<u64>,
    },
    #[returns(Status)]
    Status {},
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ucs03ExecuteMsg {
    /// This allows us to send packet via ucs03
    Send {
        channel_id: ChannelId,
        timeout_height: Uint64,
        timeout_timestamp: Timestamp,
        salt: H256,
        instruction: Bytes,
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
pub struct Executor {
    pub address: Addr,
}

#[cw_serde]
pub struct MintTokensPayload {
    pub sender: String,
    pub staker: String,
    pub amount: Uint128,
    pub salt: String,
    pub channel_id: Option<u32>,
    pub recipient: Option<String>,
    pub recipient_channel_id: Option<u32>,
}

#[cw_serde]
pub struct BondRewardsPayload {
    pub amount: Uint128,
    pub validator: String,
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
        expected: Uint128,
        slippage: Option<Decimal>,
        recipient: Option<String>,
        recipient_channel_id: Option<u32>,
    },
    Unbond {
        amount: Uint128,
        recipient: Option<String>,
        recipient_channel_id: Option<u32>,
    },
}

#[cw_serde]
pub struct BondData {
    pub mint_amount: Uint128,
    pub delegated_amount: Uint128,
    pub total_bond_amount: Uint128,
    pub exchange_rate: Decimal,
    pub total_supply: Uint128,
    pub reward_balance: Uint128,
    pub unclaimed_reward: Uint128,
}

#[cw_serde]
pub struct UnbondData {
    pub undelegate_amount: Uint128,
    pub delegated_amount: Uint128,
    pub reward: Uint128,
    pub exchange_rate: Decimal,
    pub total_supply: Uint128,
    pub record_id: u64,
    pub reward_balance: Uint128,
    pub unclaimed_reward: Uint128,
}

#[cw_serde]
pub struct MigrateMsg {}

pub struct InjectData {
    pub prev_exchange_rate: Decimal,
    pub new_exchange_rate: Decimal,
    pub total_supply: Uint128,
    pub reward_balance: Uint128,
    pub unclaimed_reward: Uint128,
    pub delegated_amount: Uint128,
    pub total_bond_amount: Uint128,
}
