use cosmwasm_std::{Decimal, Deps, StdError, Uint128};
use prost::Message;

use crate::{error::ContractError, state::State};

pub fn compute_mint_amount(
    total_bonded_native_tokens: Uint128,
    total_issued_lst: Uint128,
    native_to_stake: Uint128,
) -> Uint128 {
    // Possible truncation issues when quantities are small
    // Initial very large total_bonded_native_tokens would cause round to 0 and block minting
    // Mint at a 1:1 ratio if there is no total native token
    // Amount = Total stTIA * (Amount of native token / Total native token)
    if total_bonded_native_tokens.is_zero() {
        native_to_stake
    } else {
        total_issued_lst.multiply_ratio(native_to_stake, total_bonded_native_tokens)
    }
}

pub fn compute_unbond_amount(
    total_bonded_native_tokens: Uint128,
    total_issued_lst: Uint128,
    batch_liquid_stake_token: Uint128,
) -> Uint128 {
    if batch_liquid_stake_token.is_zero() {
        Uint128::zero()
    } else {
        // unbond amount is calculated at the batch level
        // total_bonded_native_tokens - total TIA delegated by MilkyWay
        // batch_liquid_stake_token - total stTIA in submitted batch
        // total_issued_lst - total stTIA minted by MilkyWay

        total_bonded_native_tokens.multiply_ratio(batch_liquid_stake_token, total_issued_lst)
    }
}

pub fn get_rates(state: &State) -> (Decimal, Decimal) {
    let total_bonded_native_tokens = state.total_bonded_native_tokens;
    let total_issued_lst = state.total_issued_lst;
    if total_issued_lst.is_zero() || total_bonded_native_tokens.is_zero() {
        (Decimal::one(), Decimal::one())
    } else {
        // return redemption_rate, purchase_rate
        (
            Decimal::from_ratio(total_bonded_native_tokens, total_issued_lst),
            Decimal::from_ratio(total_issued_lst, total_bonded_native_tokens),
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
