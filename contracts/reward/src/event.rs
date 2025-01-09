use cosmwasm_std::{Decimal, Event, Uint128};

pub const SPLIT_REWARD_EVENT: &str = "split_reward";

#[allow(non_snake_case)]
pub fn SplitRewardEvent(
    fee_rate: Decimal,
    split_amount: Uint128,
    redelegate_amount: Uint128,
    fee_amount: Uint128,
) -> Event {
    Event::new(SPLIT_REWARD_EVENT.to_string())
        .add_attribute("fee_rate", fee_rate.to_string())
        .add_attribute("split_amount", split_amount.to_string())
        .add_attribute("redelegate_amount", redelegate_amount.to_string())
        .add_attribute("fee_amount", fee_amount.to_string())
}
