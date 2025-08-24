use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use unionlabs_primitives::Bytes;

#[cw_serde]
pub struct FungibleLane {
    pub counterparty_beneficiary: Bytes,
}

/// (U256, ChannelId, Bytes)
pub const FUNGIBLE_COUNTERPARTY: Map<(String, u32, String), FungibleLane> =
    Map::new("fungible_counterparty");

pub const ZKGM: Item<Addr> = Item::new("zkgm");
