use cosmwasm_std::{Decimal, Event, Uint128};

pub const BOND_EVENT: &str = "bond";

#[allow(non_snake_case)]
pub fn BondEvent(
    sender: String,
    staker: String,
    bond_amount: Uint128,
    delegated_amount: Uint128,
    total_bond_amount: Uint128,
    total_supply: Uint128,
    exchange_rate: Decimal,
) -> Event {
    Event::new(BOND_EVENT.to_string())
        .add_attribute("sender", sender)
        .add_attribute("staker", staker)
        .add_attribute("bond_amount", bond_amount)
        .add_attribute("delegated_amount", delegated_amount)
        .add_attribute("total_bond_amount", total_bond_amount)
        .add_attribute("total_supply", total_supply)
        .add_attribute("exchange_rate", exchange_rate.atomics().to_string())
}

pub const UNBOND_EVENT: &str = "unbond";

#[allow(non_snake_case)]
pub fn UnbondEvent(
    sender: String,
    staker: String,
    unbond_amount: Uint128,
    delegated_amount: Uint128,
    total_bond_amount: Uint128,
    total_supply: Uint128,
    exchange_rate: Decimal,
) -> Event {
    Event::new(BOND_EVENT.to_string())
        .add_attribute("sender", sender)
        .add_attribute("staker", staker)
        .add_attribute("unbond_amount", unbond_amount)
        .add_attribute("delegated_amount", delegated_amount)
        .add_attribute("total_bond_amount", total_bond_amount)
        .add_attribute("total_supply", total_supply)
        .add_attribute("exchange_rate", exchange_rate.atomics().to_string())
}
