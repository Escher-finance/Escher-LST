use crate::state::Validator;
use cosmwasm_std::{attr, Attribute, Decimal, Event, Timestamp, Uint128};

pub const BOND_EVENT: &str = "bond";

#[allow(non_snake_case)]
pub fn BondEvent(
    sender: String,
    staker: String,
    bond_amount: Uint128,
    delegated_amount: Uint128,
    minted_amount: Uint128,
    total_bond_amount: Uint128,
    total_supply: Uint128,
    exchange_rate: Decimal,
    channel_id: String,
    time: Timestamp,
    denom: String,
) -> Event {
    Event::new(BOND_EVENT.to_string())
        .add_attribute("sender", sender)
        .add_attribute("staker", format!("{}", staker))
        .add_attribute("channel_id", channel_id)
        .add_attribute("bond_amount", bond_amount)
        .add_attribute("output_amount", minted_amount)
        .add_attribute("delegated_amount", delegated_amount)
        .add_attribute("total_bond_amount", total_bond_amount)
        .add_attribute("total_supply", total_supply)
        .add_attribute("exchange_rate", exchange_rate.atomics().to_string())
        .add_attribute("time", format!("{}", time.nanos()))
        .add_attribute("denom", denom)
}

pub const UNBOND_EVENT: &str = "unbond";

#[allow(non_snake_case)]
pub fn UnbondEvent(
    sender: String,
    staker: String,
    channel_id: Option<u32>,
    unbond_amount: Uint128,
    undelegate_amount: Uint128,
    delegated_amount: Uint128,
    total_bond_amount: Uint128,
    total_supply: Uint128,
    exchange_rate: Decimal,
    time: Timestamp,
    denom: String,
) -> Event {
    let mut channel_id_str = "".to_string();

    if channel_id.is_some() {
        channel_id_str = format!("{}", channel_id.unwrap());
    }

    Event::new(UNBOND_EVENT.to_string())
        .add_attribute("sender", sender)
        .add_attribute("staker", staker)
        .add_attribute("channel_id", channel_id_str)
        .add_attribute("unbond_amount", unbond_amount)
        .add_attribute("output_amount", undelegate_amount)
        .add_attribute("delegated_amount", delegated_amount)
        .add_attribute("total_bond_amount", total_bond_amount)
        .add_attribute("total_supply", total_supply)
        .add_attribute("exchange_rate", exchange_rate.atomics().to_string())
        .add_attribute("time", format!("{}", time.nanos()))
        .add_attribute("denom", denom)
}

pub const UPDATE_VALIDATORS_EVENT: &str = "update_validators";

#[allow(non_snake_case)]
pub fn UpdateValidatorsEvent(
    sender: String,
    prev_validators: Vec<Validator>,
    new_validators: Vec<Validator>,
) -> Event {
    let mut attrs: Vec<Attribute> = vec![];
    attrs.push(attr("sender", sender));

    for val in prev_validators.into_iter() {
        attrs.push(attr("prev_validator_addr", val.address));
        attrs.push(attr("prev_validator_weight", val.weight.to_string()));
    }

    for val in new_validators.into_iter() {
        attrs.push(attr("new_validator_addr", val.address));
        attrs.push(attr("new_validator_weight", val.weight.to_string()));
    }

    Event::new(UPDATE_VALIDATORS_EVENT.to_string()).add_attributes(attrs)
}

pub const PROCESS_REWARDS_EVENT: &str = "process_rewards";

#[allow(non_snake_case)]
pub fn ProcessRewardsEvent(total_amount: Uint128) -> Event {
    Event::new(PROCESS_REWARDS_EVENT.to_string())
        .add_attribute("total_amount", total_amount.to_string())
}

pub const PROCESS_UNBONDING_EVENT: &str = "process_unbonding";

#[allow(non_snake_case)]
pub fn ProcessUnbondingEvent(staker: String, amount: Uint128, denom: String) -> Event {
    Event::new(PROCESS_UNBONDING_EVENT.to_string())
        .add_attribute("staker", staker)
        .add_attribute("amount", amount.to_string())
        .add_attribute("denom", denom)
}
