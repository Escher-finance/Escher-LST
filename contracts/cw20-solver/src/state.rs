use cosmwasm_schema::cw_serde;
use cw_storage_plus::Map;
use ibc_union_spec::ChannelId;
use unionlabs_primitives::{Bytes, U256};

#[cw_serde]
pub struct FungibleLane {
    pub counterparty_beneficiary: Bytes,
}

pub const FUNGIBLE_COUNTERPARTY: Map<(U256, ChannelId, Bytes), FungibleLane> =
    Map::new("fungible_counterparty");
