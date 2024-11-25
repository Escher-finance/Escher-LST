use crate::token_factory_api::TokenFactoryMsg;
use crate::{msg::UndelegationRecord, state::Validator};
use cosmwasm_std::{
    Coin, CosmosMsg, Decimal, DelegationTotalRewardsResponse, QuerierWrapper, StakingMsg,
    StdResult, Uint128, Uint256,
};
use std::str::FromStr;
// to_json_binary, GrpcQuery, QueryRequest,

const DECIMAL_FRACTIONAL: u128 = 1_000_000_000_000_000_000u128;

/// return how much staking token from underlying native coin denom
pub fn calculate_staking_token_from_rate(stake_amount: Uint128, exchange_rate: Decimal) -> Uint128 {
    let decimal_fract = Decimal::new(Uint128::from(DECIMAL_FRACTIONAL * DECIMAL_FRACTIONAL));
    let fract = (exchange_rate * decimal_fract).to_uint_ceil();
    Decimal::from_ratio(Uint128::from(DECIMAL_FRACTIONAL) * stake_amount, fract).to_uint_floor()
}

/// return how much underlying native coin denom from staking token base on exchange rate
pub fn calculate_native_token_from_staking_token(
    staking_token: Uint128,
    exchange_rate: Decimal,
) -> Uint128 {
    let decimal_fract = Decimal::new(Uint128::from(DECIMAL_FRACTIONAL * DECIMAL_FRACTIONAL));
    let fract = (exchange_rate * decimal_fract).to_uint_ceil();
    Decimal::from_ratio(fract * staking_token, Uint128::from(DECIMAL_FRACTIONAL)).to_uint_floor()
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

    total
}

/// get total delegated token value from validators in native token
pub fn get_actual_total_reward(
    querier: QuerierWrapper,
    delegator: String,
    denom: String,
    validators: Vec<String>,
) -> StdResult<Uint128> {
    let mut total_rewards = Uint128::new(0);
    let result: StdResult<DelegationTotalRewardsResponse> =
        querier.query_delegation_total_rewards(delegator);

    if result.is_ok() {
        for delegator_reward in result.unwrap().rewards {
            if validators.contains(&delegator_reward.validator_address) {
                for reward in delegator_reward.reward {
                    if reward.denom == denom {
                        let reward_val = to_uint128(reward.amount.to_uint_floor())?;
                        total_rewards += reward_val;
                    }
                }
            }
        }
    }

    Ok(total_rewards)
}

pub fn to_uint128(v: Uint256) -> StdResult<Uint128> {
    Uint128::from_str(&v.to_string())
}

pub fn get_mock_total_reward(total_bond_amount: Uint128) -> Uint128 {
    let ratio = Decimal::from_ratio(Uint128::new(1000), Uint128::new(1005));
    calculate_staking_token_from_rate(total_bond_amount, ratio)
}

/// return how much to undelegate native token from ratio of total delegated amount divide with total bond with reward value amount
pub fn calculate_undelegate_amount(
    native_token_amount: Uint128,
    delegated_amount: Uint128,
    total_bonded_amount: Uint128,
) -> Uint128 {
    let native_token_undelegate_decimal =
        Decimal::new(native_token_amount * Uint128::from(DECIMAL_FRACTIONAL));
    let ratio = Decimal::from_ratio(delegated_amount, total_bonded_amount);

    println!(
        "native_token_undelegate_decimal: {:?}",
        native_token_undelegate_decimal
    );
    println!("ratio: {:?}", ratio);

    let undelegate_native_decimal = native_token_undelegate_decimal * ratio;
    undelegate_native_decimal.to_uint_floor()
}

pub fn split_revenue(amount: Uint128, fee_rate: Decimal) -> (Uint128, Uint128) {
    let decimal_fract = Decimal::new(Uint128::from(DECIMAL_FRACTIONAL * DECIMAL_FRACTIONAL));
    let fract = (fee_rate * decimal_fract).to_uint_ceil();
    let fee_amount =
        Decimal::from_ratio(fract * amount, Uint128::from(DECIMAL_FRACTIONAL)).to_uint_floor();
    let restake_amount = amount - fee_amount;
    (restake_amount, fee_amount)
}

pub fn calculate_delegated_amount(amount: Uint128, fee_rate: Decimal) -> Uint128 {
    let decimal_fract = Decimal::new(Uint128::from(DECIMAL_FRACTIONAL * DECIMAL_FRACTIONAL));
    let fract = (fee_rate * decimal_fract).to_uint_ceil();
    let fee_amount =
        Decimal::from_ratio(fract * amount, Uint128::from(DECIMAL_FRACTIONAL)).to_uint_floor();
    fee_amount
}

pub fn get_burn_msg(denom: String, burn_amount: Uint128, delegator: String) -> TokenFactoryMsg {
    let burn_msg = TokenFactoryMsg::BurnTokens {
        denom: denom.clone(),
        amount: burn_amount,
        burn_from_address: delegator,
    };

    burn_msg
}

pub fn get_undelegate_from_validator_msgs(
    undelegate_amount: Uint128,
    coin_denom: String,
    validators: Vec<Validator>,
) -> (Vec<CosmosMsg<TokenFactoryMsg>>, Vec<UndelegationRecord>) {
    let mut msgs: Vec<CosmosMsg<TokenFactoryMsg>> = vec![];
    let mut undelegations: Vec<UndelegationRecord> = vec![];

    let total_weight = Uint128::from(
        validators
            .iter()
            .map(|v| v.weight)
            .reduce(|a, b| (a + b))
            .unwrap_or(0),
    );

    let total_validators = validators.len();
    let mut total_undelegated: Uint128 = Uint128::from(0u32);

    for (pos, validator) in validators.into_iter().enumerate() {
        let ratio = Decimal::from_ratio(Uint128::from(validator.weight), total_weight);

        let undelegate_amount_dec =
            Decimal::new(undelegate_amount * Uint128::from(DECIMAL_FRACTIONAL));
        let mut undelegate_amount_for_validator = (undelegate_amount_dec * ratio).to_uint_floor();
        total_undelegated += undelegate_amount_for_validator;

        if pos == (total_validators - 1) {
            let remaining = undelegate_amount - total_undelegated;
            undelegate_amount_for_validator += remaining;
        }

        let amount = Coin {
            amount: undelegate_amount_for_validator.clone(),
            denom: coin_denom.to_string(),
        };
        let undelegate_staking_msg: CosmosMsg<TokenFactoryMsg> =
            CosmosMsg::Staking(StakingMsg::Undelegate {
                validator: validator.address.to_string(),
                amount,
            });
        msgs.push(undelegate_staking_msg);

        undelegations.push(UndelegationRecord {
            amount: undelegate_amount_for_validator,
            validator,
        })
    }

    (msgs, undelegations)
}
