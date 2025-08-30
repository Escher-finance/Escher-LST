use cosmwasm_std::{Decimal, Deps, StdError, Uint128};
use prost::Message;

use crate::{error::ContractError, state::State};

pub fn compute_mint_amount(
    total_native_token: Uint128,
    total_liquid_stake_token: Uint128,
    native_to_stake: Uint128,
) -> Uint128 {
    // Possible truncation issues when quantities are small
    // Initial very large total_native_token would cause round to 0 and block minting
    // Mint at a 1:1 ratio if there is no total native token
    // Amount = Total stTIA * (Amount of native token / Total native token)
    if total_native_token.is_zero() {
        native_to_stake
    } else {
        total_liquid_stake_token.multiply_ratio(native_to_stake, total_native_token)
    }
}

pub fn compute_unbond_amount(
    total_native_token: Uint128,
    total_liquid_stake_token: Uint128,
    batch_liquid_stake_token: Uint128,
) -> Uint128 {
    if batch_liquid_stake_token.is_zero() {
        Uint128::zero()
    } else {
        // unbond amount is calculated at the batch level
        // total_native_token - total TIA delegated by MilkyWay
        // batch_liquid_stake_token - total stTIA in submitted batch
        // total_liquid_stake_token - total stTIA minted by MilkyWay

        total_native_token.multiply_ratio(batch_liquid_stake_token, total_liquid_stake_token)
    }
}

pub fn get_rates(state: &State) -> (Decimal, Decimal) {
    let total_native_token = state.total_native_token;
    let total_liquid_stake_token = state.total_bonded_lst;
    if total_liquid_stake_token.is_zero() || total_native_token.is_zero() {
        (Decimal::one(), Decimal::one())
    } else {
        // return redemption_rate, purchase_rate
        (
            Decimal::from_ratio(total_native_token, total_liquid_stake_token),
            Decimal::from_ratio(total_liquid_stake_token, total_native_token),
        )
    }
}

/// Query the unbonding period from the chain, and verify that the batch period is smaller than the queried unbonding period.
pub fn query_and_validate_unbonding_period(
    deps: Deps,
    batch_period: u64,
) -> Result<u64, ContractError> {
    #[derive(Clone, PartialEq, Message)]
    pub struct QueryParamsRequest {}

    #[derive(Clone, PartialEq, Message)]
    pub struct QueryParamsResponse {
        #[prost(message, optional, tag = "1")]
        pub params: Option<Params>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct Params {
        #[prost(message, optional, tag = "1")]
        pub unbonding_time: Option<Duration>,
    }
    #[derive(Clone, PartialEq, Message)]
    pub struct Duration {
        #[prost(int64, tag = "1")]
        pub seconds: i64,
    }

    let res = deps.querier.query_grpc(
        "/cosmos.staking.v1beta1.Query/Params".to_owned(),
        QueryParamsRequest {}.encode_to_vec().into(),
    )?;

    let unbonding_period = QueryParamsResponse::decode(&*res)
        .map_err(|e| StdError::generic_err(format!("error decoding query params response: {e}")))
        .and_then(|res| {
            res.params.ok_or_else(|| {
                StdError::generic_err("invalid query params response, missing params")
            })
        })
        .and_then(|res| {
            res.unbonding_time.ok_or_else(|| {
                StdError::generic_err(
                    "invalid query params response, missing params.unbonding_time",
                )
            })
        })
        .and_then(|res| {
            res.seconds.try_into().map_err(|_| {
                StdError::generic_err(
                    "invalid query params response, params.unbonding_time.seconds is negative",
                )
            })
        })?;

    // Ensure the batch period is lower then unbonding period.
    if batch_period > unbonding_period {
        Err(ContractError::BatchPeriodLargerThanUnbondingPeriod {
            batch_period,
            unbonding_period,
        })
    } else {
        Ok(unbonding_period)
    }
}
