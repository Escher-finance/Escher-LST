use cosmwasm_std::{Addr, Decimal, Event, Timestamp, Uint128};

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

pub const UPDATE_CONFIG_EVENT: &str = "update_config";

#[allow(non_snake_case)]
pub fn UpdateConfigEvent(
    lst_contract_address: Addr,
    fee_receiver: Addr,
    fee_rate: Decimal,
    coin_denom: String,
) -> Event {
    Event::new(UPDATE_CONFIG_EVENT.to_string())
        .add_attribute("lst_contract_address", lst_contract_address)
        .add_attribute("fee_receiver", fee_receiver)
        .add_attribute("fee_rate", fee_rate.to_string())
        .add_attribute("coin_denom", coin_denom)
}
