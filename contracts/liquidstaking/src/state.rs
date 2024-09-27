use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};
use cw_storage_plus::Item;

pub const PARAMETERS: Item<Parameters> = Item::new("parameters");
pub const STATE: Item<State> = Item::new("state");
pub const VALIDATORS_REGISTRY: Item<ValidatorsRegistry> = Item::new("validators_registry");

pub const BALANCE: Item<Uint128> = Item::new("balance");

#[cw_serde]
pub struct Validator {
    pub address: String,
}

#[cw_serde]
pub struct State {
    pub exchange_rate: Decimal,
    // total native token plus staking rewards
    pub total_bond_amount: Uint128,
    // total native token that is delegated, include rewards
    pub total_delegated_amount: Uint128,
    // total liquid staking token that is issued
    pub total_lst_supply: Uint128,
    // bond_counter how many times bond is called
    pub bond_counter: u64,
    // last_bond_time
    pub last_bond_time: u64,
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
    pub liquidstaking_denom: String,
    pub ucs01_channel: String,
    pub ucs01_relay_contract: String,
}

impl State {
    pub fn update_exchange_rate(&mut self) {
        self.exchange_rate = Decimal::from_ratio(self.total_bond_amount, self.total_lst_supply);
    }
}
