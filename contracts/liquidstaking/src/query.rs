use crate::msg::QueryMsg;
use crate::state::{Parameters, State, ValidatorsRegistry, PARAMETERS, STATE, VALIDATORS_REGISTRY};
use cosmwasm_std::{entry_point, to_json_binary};
use cosmwasm_std::{Binary, Deps, Env, StdResult, Storage};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::State {} => to_json_binary(&query_state(deps.storage)?),
        QueryMsg::Parameters {} => to_json_binary(&query_params(deps.storage)?),
        QueryMsg::Validators {} => to_json_binary(&query_validators(deps.storage)?),
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
