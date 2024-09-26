use crate::msg::{QueryMsg, TotalBond};
use crate::state::{Parameters, State, ValidatorsRegistry, PARAMETERS, STATE, VALIDATORS_REGISTRY};
use crate::utils::{get_actual_total_bonded, get_actual_total_reward};
use cosmwasm_std::{entry_point, to_json_binary};
use cosmwasm_std::{Binary, Deps, Env, StdResult, Storage};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::State {} => to_json_binary(&query_state(deps.storage)?),
        QueryMsg::Parameters {} => to_json_binary(&query_params(deps.storage)?),
        QueryMsg::Validators {} => to_json_binary(&query_validators(deps.storage)?),
        QueryMsg::TotalBondAmount {
            delegator,
            denom,
            validators,
        } => to_json_binary(&query_total_staked_amount(
            deps, delegator, denom, validators,
        )?),
    }
}

pub fn query_state(storage: &dyn Storage) -> StdResult<State> {
    let state = STATE.load(storage)?;

    Ok(state)
}

pub fn query_params(storage: &dyn Storage) -> StdResult<Parameters> {
    let params = PARAMETERS.load(storage)?;

    Ok(params)
}

pub fn query_validators(storage: &dyn Storage) -> StdResult<ValidatorsRegistry> {
    let validators = VALIDATORS_REGISTRY.load(storage)?;

    Ok(validators)
}

pub fn query_total_staked_amount(
    deps: Deps,
    delegator: String,
    coin_denom: String,
    validators_list: Vec<String>,
) -> StdResult<TotalBond> {
    let delegated_amount = get_actual_total_bonded(
        deps.querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list.clone(),
    );

    let total_reward = get_actual_total_reward(
        deps.querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list,
    );

    Ok(TotalBond {
        amount: delegated_amount + total_reward,
    })
}
