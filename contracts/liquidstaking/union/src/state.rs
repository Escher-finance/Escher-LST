//! # Changes from original implementation
//!
//! - upgraded to cosmwasm 2
//! - removed `NativeChainConfig` as this contract is now running on the chain where staking is happening

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, DepsMut, StdError, Timestamp, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, UniqueIndex};
use unionlabs_primitives::Bytes;

use crate::{
    error::{ContractError, ContractResult},
    types::Batch,
};

#[cw_serde]
pub struct Config {
    /// The denom of the native token to liquid stake against.
    ///
    /// for `eU`, this will be `au` (the base denom of `U`).
    pub native_token_denom: String,

    /// Minimum amount of token that can be liquid staked.
    pub minimum_liquid_stake_amount: Uint128,

    /// Denomination of the liquid staking token that have been
    /// minted through the cw20.
    ///
    /// NOTE: This used to be the tokenfactory denom in the original implementation, however since eU is a cw20 token this is now the address of that cw20.
    pub liquid_stake_token_address: String,

    /// Accounts that can execute the [crate::msg::ExecuteMsg::CircuitBreaker].
    pub monitors: Vec<Addr>,

    /// Time in seconds between each batch.
    pub batch_period: u64,

    /// If true, the contract is stopped and no actions are allowed.
    pub stopped: bool,

    pub ucs03_zkgm_address: Addr,

    /// The address of the funded-dispatch contract. This is what enables the zkgm cross chain funded-dispatch staking via `Batch[TokenOrder(to=funded-dispatch), Call(funded-dispatch)]` for unstaking.
    // TODO: Consider building a more generic "allowances" system for unstaking?
    pub funded_dispatch_address: Addr,

    /// Address of the account that is performing the delegation.
    pub staker_address: Addr,

    /// Config related to the fees collected by the contract to
    /// operate the liquid staking protocol.
    pub protocol_fee_config: ProtocolFeeConfig,
}

/// Config related to the fees collected by the contract to
/// operate the liquid staking protocol.
#[cw_serde]
pub struct ProtocolFeeConfig {
    // NOTE: Previously called `dao_treasury_fee`
    pub fee_rate: Uint128, // not using a fraction, fee percentage=x/100000

    /// Address where the collected fees are sent.
    pub fee_recipient: Addr,
}

#[cw_serde]
pub struct State {
    /// The total amount of native tokens that have been bonded.
    pub total_bonded_native_tokens: Uint128,
    /// The total issued supply of the minted LST token.
    ///
    /// Note that this is *not* the same as the total supply of the LST contract, but rather the total *cross-chain* supply of the LST. When the LST is bridged, it will be burned on the source chain and minted on the destination chain.
    pub total_issued_lst: Uint128,

    // REVIEW: Unused? If this is only used for off-chain actions/ accounting then this is probably better off in a separate storage
    pub total_reward_amount: Uint128,

    // REVIEW: Pull this out into a separate storage item, no need to load it every time we read the state and also makes more sense semantically to have it separate
    pub pending_owner: Option<Addr>,
    // TODO: I think this needs to be part of an enum with the pending owner
    pub owner_transfer_min_time: Option<Timestamp>,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const ADMIN: Admin = Admin::new("admin");
pub const STATE: Item<State> = Item::new("state");
pub const BATCHES: Map<u64, Batch> = Map::new("batches");
pub const PENDING_BATCH_ID: Item<u64> = Item::new("pending_batch_id");

#[cw_serde]
pub struct UnstakeRequest {
    pub batch_id: u64,
    pub user: String,
    pub amount: Uint128,
    // TODO:
    //
    // Withdrawing unstaked tokens aggregates over (user, recipient_channel_id).
    //
    // If a user stakes 400, then unstakes:
    //
    // 100 to channel 1
    // 100 to channel 2
    // 100 to channel 1
    // 100 to no channel
    //
    // The user would then receive 200 on channel 1, 100 on channel 2, and 100 on the host chain.
    // pub recipient_channel_id: Option<ChannelId>,
}

pub struct UnstakeRequestIndexes<'a> {
    pub by_user: UniqueIndex<'a, (String, u64), UnstakeRequest, ()>,
}

impl<'a> IndexList<UnstakeRequest> for UnstakeRequestIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<UnstakeRequest>> + '_> {
        let v: Vec<&dyn Index<UnstakeRequest>> = vec![&self.by_user];
        Box::new(v.into_iter())
    }
}

pub fn unstake_requests<'a>() -> IndexedMap<(u64, String), UnstakeRequest, UnstakeRequestIndexes<'a>>
{
    let indexes = UnstakeRequestIndexes {
        by_user: UniqueIndex::new(|r| (r.user.clone(), r.batch_id), "unstake_requests_by_user"),
    };

    IndexedMap::new("unstake_requests", indexes)
}

pub fn new_unstake_request(
    deps: &mut DepsMut,
    user: String,
    batch_id: u64,
    amount: Uint128,
) -> Result<(), StdError> {
    unstake_requests().save(
        deps.storage,
        (batch_id, user.clone()),
        &UnstakeRequest {
            batch_id,
            user,
            amount,
        },
    )?;
    Ok(())
}

pub fn remove_unstake_request(
    deps: &mut DepsMut,
    user: String,
    batch_id: u64,
) -> Result<(), StdError> {
    unstake_requests()
        .remove(deps.storage, (batch_id, user.clone()))
        .unwrap();
    Ok(())
}

pub const MIGRATING: Item<bool> = Item::new("migrating");

/// Checks if the contract is being migrated.
pub fn assert_not_migrating(deps: Deps) -> ContractResult<()> {
    if MIGRATING.may_load(deps.storage)?.unwrap_or(false) {
        Err(ContractError::Migrating {})
    } else {
        Ok(())
    }
}

/// Map of source channel ids to quote token config on
pub const FUNGIBLE_RECIPIENT_CHANNEL: Map<u32, CounterpartyConfig> =
    Map::new("idkwhattocallthisyet");

#[cw_serde]
pub struct CounterpartyConfig {
    pub quote_token: Bytes,
    pub kind: u8,
    pub metadata: Bytes,
}
