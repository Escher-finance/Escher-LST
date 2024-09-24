use cosmwasm_std::{
    Decimal, DelegationTotalRewardsResponse, QuerierWrapper, StdError, StdResult, Uint128,
};
use std::str::FromStr;

const DECIMAL_FRACTIONAL: u128 = 1_000_000_000_000_000_000u128;

/// return a / b
pub fn decimal_division(a: Uint128, b: Decimal) -> Uint128 {
    let fract = Decimal::new(Uint128::new(DECIMAL_FRACTIONAL)) * b;
    let total = Uint128::new(fract.to_uint_ceil().into());
    let mut decimal = Decimal::from_ratio(a, total);
    decimal = decimal * (Decimal::new(Uint128::from(DECIMAL_FRACTIONAL)));
    return decimal.to_uint_ceil();
}

/// get total delegated token value from validators in native token
pub fn get_actual_total_bonded(
    querier: QuerierWrapper,
    delegator: String,
    denom: String,
    validators: Vec<String>,
) -> Uint128 {
    let delegations_resp = querier.query_all_delegations(delegator);
    let mut total: Uint128 = Uint128::new(0);

    if delegations_resp.is_ok() {
        total = delegations_resp
            .unwrap()
            .into_iter()
            .filter(|d| {
                d.amount.denom == denom
                    && !d.amount.amount.is_zero()
                    && validators.contains(&d.validator)
            })
            .map(|d| d.amount.amount)
            .sum();
    }

    return total;
}

/// get total delegated token value from validators in native token
pub fn get_actual_total_reward(
    querier: QuerierWrapper,
    delegator: String,
    denom: String,
    validators: Vec<String>,
) -> Uint128 {
    let mut total_rewards = Uint128::new(0);
    let result: StdResult<DelegationTotalRewardsResponse> =
        querier.query_delegation_total_rewards(delegator);

    if result.is_ok() {
        for delegator_reward in result.unwrap().rewards {
            if validators.contains(&delegator_reward.validator_address) {
                for reward in delegator_reward.reward {
                    if reward.denom == denom {
                        let reward_val: Result<Uint128, StdError> =
                            Uint128::from_str(reward.amount.to_string().as_str());

                        if reward_val.is_ok() {
                            total_rewards += reward_val.unwrap();
                        }
                    }
                }
            }
        }
    }

    return total_rewards;
}
