use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};
use cw_storage_plus::Item;

pub const CONFIG: Item<Config> = Item::new("config");
pub const PARAMETERS: Item<Parameters> = Item::new("parameters");
pub const STATE: Item<State> = Item::new("state");
pub const VALIDATORS_REGISTRY: Item<ValidatorsRegistry> = Item::new("validators_registry");

#[cw_serde]
pub struct Validator {
    pub address: String,
}

#[cw_serde]
pub struct State {
    pub exchange_rate: Decimal,
    pub total_bond_amount: Uint128,
    pub last_unbonded_time: u64,
}

// Config is configuration that still possible to change
#[cw_serde]
pub struct Config {
    pub validators: Vec<Validator>,
}

// Config is configuration that still possible to change
#[cw_serde]
pub struct ValidatorsRegistry {
    pub validators: Vec<Validator>,
}

// Parameter is not changeable configuration
#[cw_serde]
pub struct Parameters {
    pub underlying_coin_denom: String,
}
