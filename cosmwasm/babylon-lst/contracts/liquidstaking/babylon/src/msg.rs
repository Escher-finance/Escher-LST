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
        Parameters, QuoteToken, State, Status, SupplyQueue, UnbondRecord, Validator,
        ValidatorsRegistry,
    },
    types::ChannelId,
    utils::batch::{Batch, BatchStatus},
};

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
    /// tokens
    pub quote_tokens: Vec<QuoteToken>,
    /// epoch period
    pub epoch_period: Option<u32>,
    // batch period range in seconds to execute batch
    pub batch_period: u64,
    /// whether to use external reward contract
    /// if true, the contract will use external reward contract
    pub use_external_reward: Option<bool>,
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
        recipient_ibc_channel_id: Option<String>,
    },
}

#[cw_serde]
pub enum Recipient {
    OnChain {
        address: Addr,
    },
    Zkgm {
        address: String,
        channel_id: u32,
    },
    IBC {
        address: String,
        ibc_channel_id: String,
    },
}

#[cw_ownable_execute]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteMsg {
    /// Delegate native denom `amount` to validator
    /// Issue `amount` / `exchange_rate` for the user.
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
        ucs03_relay_contract: Option<String>,
        unbonding_time: Option<u64>,
        cw20_address: Option<Addr>,
        reward_address: Option<Addr>,
        fee_receiver: Option<Addr>,
        fee_rate: Option<Decimal>,
        batch_period: Option<u64>,
        epoch_period: Option<u32>,
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
    /// Redelegate some amount that is called from reward contract as result of split reward call to reward contract
    Redelegate {},
    /// Call migrate to reward contract
    MigrateReward {
        code_id: u64,
    },
    SplitReward {},
    SetStatus(Status),
    SetChain {
        chain: crate::state::Chain,
    },
    RemoveChain {
        channel_id: u32,
    },
    NormalizeReward {},
    Inject {
        amount: Uint128,
    },
    AddIbcChannel {
        ibc_channel_id: String,
        prefix: String,
    },
    RemoveIbcChannel {
        ibc_channel_id: String,
    },
    RemoteBond {
        min_mint_amount: Uint128,
        mint_to_address: Addr,
    },
    RemoteUnbond {
        amount: Uint128,
        recipient: Recipient,
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
    #[returns(Vec<Batch>)]
    Batch {
        id: Option<u64>,
        status: Option<BatchStatus>,
        min: Option<u64>,
        max: Option<u64>,
    },
    #[returns(SupplyQueue)]
    SupplyQueue {},
    #[returns(Status)]
    Status {},
    #[returns(Vec<cosmwasm_std::FullDelegation>)]
    Delegations {},
    #[returns(Vec<crate::state::Chain>)]
    Chains {},
    #[returns(Vec<crate::state::WithdrawRewardQueue>)]
    RewardQueue {},
    #[returns(Vec<IBCChannel>)]
    IbcChannels {},
    #[returns(IbcChannelId)]
    RecipientIbcChannel { unbond_record_id: u64 },
}

pub type Fees = BTreeMap<String, Coin>;

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Ucs03ExecuteMsg {
    /// This allows us to transfer via ucs03
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
    pub adjusted_supply: Uint128,
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
    pub transfer_fee: Option<Uint128>,
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
        expected: Uint128,
        slippage: Option<Decimal>,
        recipient: Option<String>,
        recipient_channel_id: Option<u32>,
    },
    Unbond {
        amount: Uint128,
        recipient: Option<String>,
        recipient_channel_id: Option<u32>,
        recipient_ibc_channel_id: Option<String>,
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
pub struct RemoteBondData {
    pub denom: String,
    pub bond_amount: Uint128,
    pub mint_amount: Uint128,
    pub delegated_amount: Uint128,
    pub total_bond_amount: Uint128,
    pub exchange_rate: Decimal,
    pub total_supply: Uint128,
    pub reward_balance: Uint128,
    pub unclaimed_reward: Uint128,
    pub cw20_address: String,
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
pub struct MigrateMsg {
    pub transfer_handler: Option<String>,
    pub zkgm_token_minter: Option<String>,
}

#[cw_serde]
pub struct RewardMigrateMsg {}

#[cw_serde]
pub struct IBCCallbackPayload {
    pub amount: Uint128,
    pub slippage: Option<Decimal>,
    pub expected: Uint128,
    pub salt: String,
    pub recipient: String,
    pub recipient_channel_id: Option<u32>,
    pub transfer_fee: Option<Uint128>,
}

pub struct InjectData {
    pub prev_exchange_rate: Decimal,
    pub new_exchange_rate: Decimal,
    pub total_supply: Uint128,
    pub reward_balance: Uint128,
    pub unclaimed_reward: Uint128,
    pub delegated_amount: Uint128,
    pub total_bond_amount: Uint128,
}

#[cw_serde]
pub struct IBCChannel {
    pub ibc_channel_id: String,
    pub prefix: String,
}

#[cw_serde]
pub struct IbcChannelId {
    pub channel_id: String,
}

#[cw_serde]
pub struct LiquidityState {
    pub assets: Uint128, // delegated + erward
    pub delegated: Uint128,
    pub reward_balance: Uint128,
    pub unclaimed_reward: Uint128,
    pub exchange_rate: Decimal,
}

#[derive(Debug)]
pub struct ZkgmTransfer {
    pub sender: String,
    pub amount: Uint128,
    pub recipient: String,
    pub recipient_channel_id: u32,
    pub salt: String,
}
