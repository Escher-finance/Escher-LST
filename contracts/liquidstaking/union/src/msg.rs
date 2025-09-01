use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use ibc_union_spec::ChannelId;
use on_zkgm_call_proxy::OnProxyOnZkgmCall;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use unionlabs_primitives::{Bytes, H256};

use crate::types::{
    Batch, BatchExpectedAmount, BatchId, BatchStatus, ProtocolFeeConfig, Staker, UnstakeRequest,
    UnstakeRequestKey,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstantiateMsg {
    pub native_token_denom: String,

    pub minimum_liquid_stake_amount: Uint128,

    /// Address of the account that delegates the tokens
    /// toward the validators.
    pub staker_address: Addr,

    /// Protocol fee configuration.
    pub protocol_fee_config: ProtocolFeeConfig,

    /// Address of the LST contract.
    pub lst_address: Addr,

    /// Frequency (in seconds) at which the unbonding queue is executed.
    pub batch_period_seconds: u64,

    /// Set of addresses allowed to trigger a circuit break.
    pub monitors: Vec<Addr>,
    pub admin: Addr,

    pub ucs03_zkgm_address: Addr,

    pub on_zkgm_call_proxy_address: Addr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteMsg {
    /// Initiates the bonding process for a user.
    Bond {
        /// The address to mint the LST to.
        mint_to: Addr,

        /// Minimum expected amount of LST tokens to be received
        /// for the operation to be considered valid.
        min_mint_amount: Uint128,
    },

    /// Initiates the unbonding process for a user.
    Unbond {
        /// The address that will receive the native tokens on.
        staker: Addr,

        /// The amount to unstake.
        ///
        /// NOTE: In the original milkyway implementation, the contract expected funds to be sent to it (verified with must_pay) since the tokens were all native (ibc denoms for the base token, tokenfactory for the lst)
        amount: Uint128,
    },

    /// Withdraws unstaked tokens.
    Withdraw {
        /// The address that will receive the funds.
        staker: Addr,
        /// The address to withdraw the funds to on this chain.
        withdraw_to_address: Addr,
        /// ID of the batch from which to withdraw.
        batch_id: BatchId,
    },

    /// Processes the pending batch.
    SubmitBatch {},

    TransferOwnership {
        /// Address of the new owner on the protocol chain.
        new_owner: String,
    },

    /// Accepts ownership transfer; callable by the new owner.
    AcceptOwnership {},

    /// Revokes ownership transfer; callable by the current owner.
    RevokeOwnershipTransfer {},

    // TODO: Implement once basic functionality is complete
    // /// Updates contract configuration; callable by the owner.
    // UpdateConfig {
    //     /// Updated protocol fee configuration.
    //     protocol_fee_config: Option<ProtocolFeeConfig>,

    //     /// Updated list of circuit breaker monitors.
    //     monitors: Option<Vec<String>>,

    //     /// Updated unbonding batch execution frequency (in seconds).
    //     batch_period: Option<u64>,
    // },
    /// Receives rewards from the native chain.
    ReceiveRewards {},

    /// Receives unstaked tokens from the native chain.
    ReceiveUnstakedTokens {
        /// ID of the batch that originated the unstake request.
        batch_id: BatchId,
    },

    /// Stops the contract due to irregularities; callable by monitors and admin.
    CircuitBreaker {},

    /// Resumes the contract; callable by the admin.
    ResumeContract {
        /// Updated total native tokens delegated (used post-slashing).
        total_bonded_native_tokens: Uint128,

        /// Updated total issued liquid staked tokens.
        total_issued_lst: Uint128,

        /// Updated total protocol rewards.
        total_reward_amount: Uint128,
    },
    SlashBatches {
        new_amounts: Vec<BatchExpectedAmount>,
    },
    OnProxyOnZkgmCall(OnProxyOnZkgmCall),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RemoteExecuteMsg {
    /// Initiates the bonding process for a user.
    Bond {
        /// The address to mint the LST to.
        mint_to: Addr,

        /// Minimum expected amount of LST tokens to be received for the operation to be considered valid. Any slippage will be sent to the relayer of the packet.
        min_mint_amount: Uint128,
    },

    /// Initiates the unbonding process for a user.
    Unbond {
        /// The amount to unstake.
        amount: Uint128,
    },

    /// Withdraws unstaked tokens.
    Withdraw {
        /// ID of the batch from which to withdraw.
        batch_id: BatchId,
        /// The address to withdraw the funds to on this chain.
        withdraw_to_address: Addr,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[derive(QueryResponses)]
pub enum QueryMsg {
    /// Queries the contract configuration.
    /// Returns the current `native_chain_config`, `protocol_chain_config`,
    /// `protocol_fee_config`, `liquid_stake_token_denom`, and other settings.
    // #[returns(ConfigResponse)]
    Config {},

    /// Queries the current state of the contract.
    /// Returns totals such as delegated native tokens, LST supply, and rewards.
    // #[returns(StateResponse)]
    State {},

    /// Queries the information of a specific batch by its ID.
    // #[returns(Batch)]
    Batch {
        /// ID of the batch to query.
        batch_id: BatchId,
    },

    /// Queries a paginated list of all batches stored in contract storage.
    // #[returns(BatchesResponse)]
    Batches {
        /// If provided, starts listing batches after this batch ID.
        start_after: Option<BatchId>,

        /// Maximum number of batches to return.
        limit: Option<usize>,

        /// Optional filter to return only batches with the given status.
        status: Option<BatchStatus>,
    },

    /// Queries the batches with the provided list of IDs.
    // #[returns(BatchesResponse)]
    BatchesByIds {
        /// List of batch IDs to fetch.
        batch_ids: Vec<BatchId>,
    },

    /// Queries the current batch that is pending processing (if any).
    // #[returns(Batch)]
    PendingBatch {},

    /// Queries the unstake requests made by a specific staker.
    // #[returns(Vec<UnstakeRequest>)]
    UnstakeRequestsByStaker {
        /// Address of the user whose unstake requests are to be queried.
        staker: Staker,
    },

    /// Queries the unstake requests made by a specific staker, by the staker hash.
    // #[returns(Vec<UnstakeRequest>)]
    UnstakeRequestsByStakerHash {
        /// Address of the user whose unstake requests are to be queried.
        staker_hash: H256,
    },

    /// Queries all unstake requests in the contract.
    // #[returns(Vec<UnstakeRequestResponse>)]
    AllUnstakeRequests {
        /// If provided, starts listing unstake requests after this key.
        start_after: Option<UnstakeRequestKey>,

        /// Maximum number of unstake requests to return.
        limit: Option<usize>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub native_token_denom: String,
    pub minimum_liquid_stake_amount: Uint128,
    pub protocol_fee_config: ProtocolFeeConfig,
    pub monitors: Vec<Addr>,
    pub lst_address: Addr,
    pub batch_period_seconds: u64,
    pub stopped: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountingStateResponse {
    pub total_bonded_native_tokens: Uint128,
    pub total_issued_lst: Uint128,
    pub total_reward_amount: Uint128,
    pub redemption_rate: Decimal,
    pub purchase_rate: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnershipResponse {
    pub pending_owner: String,
    pub total_reward_amount: Uint128,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchesResponse {
    pub batches: BTreeMap<BatchId, Batch>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnstakeRequestResponse {
    pub batch_id: BatchId,
    pub batch_total_liquid_stake: Uint128,
    pub expected_native_unstaked: Uint128,
    pub received_native_unstaked: Uint128,
    pub status: String,
    pub unstake_amount: Uint128,
    pub user: String,
}
