use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::Item;

pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct Config {
    pub lst_contract_address: Addr,
    pub fee_receiver: Addr,
    pub fee_rate: Decimal,
    pub coin_denom: String,
}
