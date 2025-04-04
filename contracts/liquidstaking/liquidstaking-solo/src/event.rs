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
    batch_id: u64,
    validator: String,
    undelegate_amount: String,
    time: Timestamp,
) -> Event {
    Event::new(UNBOND_EVENT.to_string())
        .add_attribute("batch_id", format!("{}", batch_id))
        .add_attribute("validator", validator)
        .add_attribute("output_amount", undelegate_amount)
        .add_attribute("time", format!("{}", time.nanos()))
}

#[allow(non_snake_case)]
pub fn UnbondEventsFromAtts(atts: Vec<Attribute>, batch_id: u64, time: Timestamp) -> Vec<Event> {
    let mut events = vec![];
    for att in atts {
        events.push(UnbondEvent(
            batch_id,
            att.key.clone(),
            att.value.clone(),
            time,
        ));
    }
    events
}

pub const SUBMIT_BATCH_EVENT: &str = "submit_batch";

#[allow(non_snake_case)]
pub fn SubmitBatchEvent(
    batch_id: u64,
    sender: String,
    unstake_amount: Uint128,
    undelegate_amount: Uint128,
    delegated_amount: Uint128,
    total_bond_amount: Uint128,
    total_supply: Uint128,
    exchange_rate: Decimal,
    time: Timestamp,
    denom: String,
) -> Event {
    Event::new(SUBMIT_BATCH_EVENT.to_string())
        .add_attribute("batch_id", format!("{}", batch_id))
        .add_attribute("sender", sender)
        .add_attribute("unstake_amount", unstake_amount)
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
pub fn ProcessUnbondingEvent(
    batch_id: u64,
    channel_id: Option<u32>,
    staker: String,
    amount: Uint128,
    denom: String,
    time: Timestamp,
) -> Event {
    Event::new(PROCESS_UNBONDING_EVENT.to_string())
        .add_attribute("staker", staker)
        .add_attribute("amount", amount.to_string())
        .add_attribute("denom", denom)
        .add_attribute("time", format!("{}", time.nanos()))
        .add_attribute("batch_id", format!("{}", batch_id))
        .add_attribute("channel_id", channel_id.unwrap_or(0).to_string())
}

pub const PROCESS_BATCH_UNBONDING_EVENT: &str = "process_batch_unbonding";

#[allow(non_snake_case)]
pub fn ProcessBatchUnbondingEvent(
    batch_id: u64,
    time: Timestamp,
    released_amount: Uint128,
    total_amount: Uint128,
    denom: String,
    record_ids: Vec<u64>,
) -> Event {
    Event::new(PROCESS_BATCH_UNBONDING_EVENT.to_string())
        .add_attribute("total_amount", total_amount.to_string())
        .add_attribute("released_amount", released_amount.to_string())
        .add_attribute("denom", denom)
        .add_attribute("time", format!("{}", time.nanos()))
        .add_attribute("batch_id", format!("{}", batch_id))
        .add_attribute(
            "record_ids",
            record_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(","),
        )
}

pub const UNSTAKE_REQUEST_EVENT: &str = "unstake_request";

#[allow(non_snake_case)]
pub fn UnstakeRequestEvent(
    sender: String,
    staker: String,
    channel_id: Option<u32>,
    amount: Uint128,
    record_id: u64,
    batch_id: u64,
    time: Timestamp,
) -> Event {
    let mut channel_id_str = "".to_string();

    if channel_id.is_some() {
        channel_id_str = format!("{}", channel_id.unwrap());
    }

    Event::new(UNSTAKE_REQUEST_EVENT.to_string())
        .add_attribute("sender", sender)
        .add_attribute("staker", staker)
        .add_attribute("channel_id", channel_id_str)
        .add_attribute("unbond_amount", amount)
        .add_attribute("time", format!("{}", time.nanos()))
        .add_attribute("batch_id", format!("{}", batch_id))
        .add_attribute("record_id", format!("{}", record_id))
}

pub const BATCH_RECEIVED_EVENT: &str = "batch_received";

#[allow(non_snake_case)]
pub fn BatchReceivedEvent(batch_id: u64, received_amount: String, time: Timestamp) -> Event {
    Event::new(UNBOND_EVENT.to_string())
        .add_attribute("batch_id", format!("{}", batch_id))
        .add_attribute("received_amount", received_amount)
        .add_attribute("time", format!("{}", time.nanos()))
}

pub const SPLIT_REWARD_EVENT: &str = "split_reward";

#[allow(non_snake_case)]
pub fn SplitRewardEvent(
    fee_rate: Decimal,
    split_amount: Uint128,
    redelegate_amount: Uint128,
    fee_amount: Uint128,
    time: Timestamp,
) -> Event {
    Event::new(SPLIT_REWARD_EVENT.to_string())
        .add_attribute("fee_rate", fee_rate.to_string())
        .add_attribute("split_amount", split_amount.to_string())
        .add_attribute("redelegate_amount", redelegate_amount.to_string())
        .add_attribute("fee_amount", fee_amount.to_string())
        .add_attribute("time", format!("{}", time.nanos()))
}
