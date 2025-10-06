use cosmwasm_std::{Attribute, Decimal, Event, Timestamp, Uint128, attr};

use crate::state::Validator;
pub const BOND_EVENT: &str = "bond";

#[allow(non_snake_case)]
#[allow(clippy::too_many_arguments)]
#[must_use]
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
    recipient: Option<String>,
    recipient_channel_id: Option<u32>,
    reward_balance: Uint128,
    unclaimed_reward: Uint128,
    ibc_channel_id: Option<String>,
) -> Event {
    let recipient = match recipient {
        Some(recipient) => recipient,
        None => String::new(),
    };

    let recipient_channel_id: String = match recipient_channel_id {
        Some(channel_id) => channel_id.to_string(),
        None => "0".to_string(),
    };

    let ibc_channel_id: String = match ibc_channel_id {
        Some(channel_id) => channel_id.to_string(),
        None => String::new(),
    };

    Event::new(BOND_EVENT.to_string())
        .add_attribute("sender", sender)
        .add_attribute("staker", staker)
        .add_attribute("channel_id", channel_id)
        .add_attribute("bond_amount", bond_amount)
        .add_attribute("output_amount", minted_amount)
        .add_attribute("delegated_amount", delegated_amount)
        .add_attribute("total_bond_amount", total_bond_amount)
        .add_attribute("total_supply", total_supply)
        .add_attribute("exchange_rate", exchange_rate.atomics().to_string())
        .add_attribute("time", format!("{}", time.nanos()))
        .add_attribute("denom", denom)
        .add_attribute("recipient", recipient)
        .add_attribute("recipient_channel_id", recipient_channel_id)
        .add_attribute("reward_balance", reward_balance)
        .add_attribute("unclaimed_reward", unclaimed_reward)
        .add_attribute("ibc_channel_id", ibc_channel_id)
}

pub const UNBOND_EVENT: &str = "unbond";

#[allow(non_snake_case)]
#[must_use]
pub fn UnbondEvent(
    batch_id: u64,
    validator: String,
    undelegate_amount: String,
    time: Timestamp,
) -> Event {
    Event::new(UNBOND_EVENT.to_string())
        .add_attribute("batch_id", format!("{batch_id}"))
        .add_attribute("validator", validator)
        .add_attribute("output_amount", undelegate_amount)
        .add_attribute("time", format!("{}", time.nanos()))
}

#[allow(non_snake_case)]
#[must_use]
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
#[allow(clippy::too_many_arguments)]
#[must_use]
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
    reward_balance: Uint128,
    unclaimed_reward: Uint128,
) -> Event {
    Event::new(SUBMIT_BATCH_EVENT.to_string())
        .add_attribute("batch_id", format!("{batch_id}"))
        .add_attribute("sender", sender)
        .add_attribute("unstake_amount", unstake_amount)
        .add_attribute("output_amount", undelegate_amount)
        .add_attribute("delegated_amount", delegated_amount)
        .add_attribute("total_bond_amount", total_bond_amount)
        .add_attribute("total_supply", total_supply)
        .add_attribute("exchange_rate", exchange_rate.atomics().to_string())
        .add_attribute("time", format!("{}", time.nanos()))
        .add_attribute("denom", denom)
        .add_attribute("reward_balance", reward_balance)
        .add_attribute("unclaimed_reward", unclaimed_reward)
}

pub const UPDATE_VALIDATORS_EVENT: &str = "update_validators";

#[allow(non_snake_case)]
#[must_use]
pub fn UpdateValidatorsEvent(
    sender: String,
    prev_validators: Vec<Validator>,
    new_validators: Vec<Validator>,
) -> Event {
    let mut attrs: Vec<Attribute> = vec![];
    attrs.push(attr("sender", sender));

    for val in prev_validators {
        attrs.push(attr("prev_validator_addr", val.address));
        attrs.push(attr("prev_validator_weight", val.weight.to_string()));
    }

    for val in new_validators {
        attrs.push(attr("new_validator_addr", val.address));
        attrs.push(attr("new_validator_weight", val.weight.to_string()));
    }

    Event::new(UPDATE_VALIDATORS_EVENT.to_string()).add_attributes(attrs)
}

pub const PROCESS_REWARDS_EVENT: &str = "process_rewards";

#[allow(non_snake_case)]
#[must_use]
pub fn ProcessRewardsEvent(total_amount: Uint128, balance_reward: Uint128) -> Event {
    Event::new(PROCESS_REWARDS_EVENT.to_string())
        .add_attribute("withdraw_reward_amount", total_amount.to_string())
        .add_attribute("balance_reward", balance_reward.to_string())
}

pub const PROCESS_UNBONDING_EVENT: &str = "process_unbonding";

#[allow(non_snake_case)]
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn ProcessUnbondingEvent(
    batch_id: u64,
    channel_id: Option<u32>,
    staker: String,
    amount: Uint128,
    denom: String,
    time: Timestamp,
    recipient: Option<String>,
    recipient_channel_id: Option<u32>,
    recipient_ibc_channel_id: Option<String>,
) -> Event {
    let recipient = match recipient {
        Some(recipient) => recipient,
        None => staker.clone(),
    };

    let recipient_channel_id: String = match recipient_channel_id {
        Some(channel_id) => channel_id.to_string(),
        None => "0".to_string(),
    };

    let recipient_ibc_channel_id: String = match recipient_ibc_channel_id {
        Some(channel_id) => channel_id.to_string(),
        None => String::new(),
    };

    Event::new(PROCESS_UNBONDING_EVENT.to_string())
        .add_attribute("staker", staker)
        .add_attribute("amount", amount.to_string())
        .add_attribute("denom", denom)
        .add_attribute("time", format!("{}", time.nanos()))
        .add_attribute("batch_id", format!("{batch_id}"))
        .add_attribute("channel_id", channel_id.unwrap_or(0).to_string())
        .add_attribute("recipient", recipient)
        .add_attribute("recipient_channel_id", recipient_channel_id)
        .add_attribute("recipient_ibc_channel_id", recipient_ibc_channel_id)
}

pub const PROCESS_BATCH_UNBONDING_EVENT: &str = "process_batch_unbonding";

#[allow(non_snake_case)]
#[must_use]
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
        .add_attribute("batch_id", format!("{batch_id}"))
        .add_attribute(
            "record_ids",
            format!(
                "[{}]",
                record_ids
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        )
}

pub const UNSTAKE_REQUEST_EVENT: &str = "unstake_request";

#[allow(non_snake_case)]
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn UnstakeRequestEvent(
    sender: String,
    staker: String,
    channel_id: Option<u32>,
    amount: Uint128,
    record_id: u64,
    batch_id: u64,
    time: Timestamp,
    recipient: Option<String>,
    recipient_channel_id: Option<u32>,
    reward_balance: Uint128,
    recipient_ibc_channel_id: Option<String>,
) -> Event {
    let channel_id: String = match channel_id {
        Some(channel_id) => channel_id.to_string(),
        None => String::new(),
    };

    let recipient = match recipient {
        Some(recipient) => recipient,
        None => String::new(),
    };

    let recipient_channel_id: String = match recipient_channel_id {
        Some(channel_id) => channel_id.to_string(),
        None => "0".to_string(),
    };

    let recipient_ibc_channel_id: String = match recipient_ibc_channel_id {
        Some(channel_id) => channel_id.to_string(),
        None => String::new(),
    };

    Event::new(UNSTAKE_REQUEST_EVENT.to_string())
        .add_attribute("sender", sender)
        .add_attribute("staker", staker)
        .add_attribute("channel_id", channel_id)
        .add_attribute("unbond_amount", amount)
        .add_attribute("time", format!("{}", time.nanos()))
        .add_attribute("batch_id", format!("{batch_id}"))
        .add_attribute("record_id", format!("{record_id}"))
        .add_attribute("recipient", recipient)
        .add_attribute("recipient_channel_id", recipient_channel_id)
        .add_attribute("reward_balance", reward_balance)
        .add_attribute("recipient_ibc_channel_id", recipient_ibc_channel_id)
}

pub const BATCH_RECEIVED_EVENT: &str = "batch_received";

#[allow(non_snake_case)]
#[must_use]
pub fn BatchReceivedEvent(batch_id: u64, received_amount: String, time: Timestamp) -> Event {
    Event::new(BATCH_RECEIVED_EVENT.to_string())
        .add_attribute("batch_id", format!("{batch_id}"))
        .add_attribute("received_amount", received_amount)
        .add_attribute("time", format!("{}", time.nanos()))
}

pub const SPLIT_REWARD_EVENT: &str = "split_reward";

#[allow(non_snake_case)]
#[must_use]
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

pub const BATCH_RELEASED_EVENT: &str = "batch_released";

#[allow(non_snake_case)]
pub fn BatchReleasedEvent(batch_id: u64, time: Timestamp) -> Event {
    Event::new(BATCH_RELEASED_EVENT.to_string())
        .add_attribute("batch_id", format!("{}", batch_id))
        .add_attribute("time", format!("{}", time.nanos()))
}

pub const INJECT_EVENT: &str = "inject";

#[allow(non_snake_case)]
#[allow(clippy::too_many_arguments)]
pub fn InjectEvent(
    amount: Uint128,
    reward_balance: Uint128,
    unclaimed_reward: Uint128,
    prev_exchange_rate: Decimal,
    exchange_rate: Decimal,
    delegated_amount: Uint128,
    total_bond_amount: Uint128,
    total_supply: Uint128,
    time: Timestamp,
) -> Event {
    Event::new(INJECT_EVENT.to_string())
        .add_attribute("amount", amount)
        .add_attribute("reward_balance", reward_balance)
        .add_attribute("unclaimed_reward", unclaimed_reward.to_string())
        .add_attribute("prev_exchange_rate", prev_exchange_rate.to_string())
        .add_attribute("exchange_rate", exchange_rate.to_string())
        .add_attribute("delegated_amount", delegated_amount.to_string())
        .add_attribute("total_bond_amount", total_bond_amount.to_string())
        .add_attribute("total_supply", total_supply.to_string())
        .add_attribute("time", format!("{}", time.nanos()))
}

pub const IBC_CALLBACK_EVENT: &str = "ibc_callback";

#[allow(non_snake_case)]
#[allow(clippy::too_many_arguments)]
pub fn IbcCallbackEvent(
    sender: String,
    ibc_channel_id: String,
    transfer_amount: Uint128,
    amount: Uint128,
    recipient: String,
    recipient_channel_id: Option<u32>,
    salt: String,
    bond_status: bool,
    error_message: String,
    time: Timestamp,
    transfer_fee: Uint128,
) -> Event {
    let recipient_channel_id: String = match recipient_channel_id {
        Some(channel_id) => channel_id.to_string(),
        None => "0".to_string(),
    };

    Event::new(IBC_CALLBACK_EVENT.to_string())
        .add_attribute("sender", sender)
        .add_attribute("ibc_channel_id", ibc_channel_id)
        .add_attribute("transfer_amount", transfer_amount)
        .add_attribute("amount", amount)
        .add_attribute("salt", salt)
        .add_attribute("recipient", recipient)
        .add_attribute("recipient_channel_id", recipient_channel_id)
        .add_attribute("bond_status", bond_status.to_string())
        .add_attribute("error_message", error_message)
        .add_attribute("time", format!("{}", time.nanos()))
        .add_attribute("transfer_fee", transfer_fee.to_string())
}
