use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};
use cw_storage_plus::Item;

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
    // total native token value that is delegated
    pub total_bond_amount: Uint128,
    // total liquid staking token that is issued
    pub total_lst_supply: Uint128,
    pub last_unbonded_time: u64,
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
    pub staked_token_denom: String,
    pub staked_token_denom_address: String,
}

impl State {
    pub fn update_exchange_rate(&mut self, total_issued: Uint128, requested: Uint128) {
        let actual_supply = total_issued + requested;
        if self.total_bond_amount.is_zero() || actual_supply.is_zero() {
            self.exchange_rate = Decimal::one()
        } else {
            self.exchange_rate = Decimal::from_ratio(self.total_bond_amount, actual_supply);
        }
    }
}
