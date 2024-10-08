use crate::msg::{Log, QueryMsg, TotalBond};
use crate::state::unbond_history;
use crate::state::{
    Balance, Parameters, State, UnbondHistory, ValidatorsRegistry, BALANCE, LOG, PARAMETERS, STATE,
    VALIDATORS_REGISTRY,
};
use crate::utils::{get_actual_total_bonded, get_actual_total_reward};
use cosmwasm_std::{entry_point, to_json_binary, Order};
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
        QueryMsg::Balance {} => to_json_binary(&(query_balance(deps.storage)?)),
        QueryMsg::Log {} => to_json_binary(&(query_log(deps.storage)?)),
        QueryMsg::UnbondHistory {
            source,
            sender,
            released,
        } => to_json_binary(&(query_unbond_history(deps.storage, source, sender, released)?)),
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

pub fn query_balance(storage: &dyn Storage) -> StdResult<Balance> {
    let balance = BALANCE.load(storage)?;
    Ok(balance)
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
    )?;

    Ok(TotalBond {
        amount: delegated_amount + total_reward,
        delegated: delegated_amount,
        reward: total_reward,
    })
}

pub fn query_log(storage: &dyn Storage) -> StdResult<Log> {
    let log = LOG.load(storage)?;
    Ok(Log { message: log })
}

pub fn query_unbond_history(
    storage: &dyn Storage,
    source: Option<String>,
    sender: Option<String>,
    released: Option<bool>,
) -> StdResult<Vec<UnbondHistory>> {
    if source.is_some() && released.is_none() {
        let unbonded_list = unbond_history()
            .idx
            .source
            .prefix(source.unwrap())
            .range(storage, None, None, Order::Ascending)
            .map(|n| n.unwrap().1)
            .collect::<Vec<_>>();

        return Ok(unbonded_list);
    }

    if source.is_some() && released.is_some() {
        let unbonded_list = unbond_history()
            .idx
            .source_released
            .prefix(format!(
                "{}-{}",
                source.unwrap(),
                released.unwrap().to_string()
            ))
            .range(storage, None, None, Order::Ascending)
            .map(|n| n.unwrap().1)
            .collect::<Vec<_>>();

        return Ok(unbonded_list);
    }

    if source.is_none() && sender.is_some() {
        let unbonded_list = unbond_history()
            .idx
            .sender
            .prefix(sender.unwrap())
            .range(storage, None, None, Order::Ascending)
            .map(|n| n.unwrap().1)
            .collect::<Vec<_>>();

        return Ok(unbonded_list);
    }

    if source.is_none() && released.is_some() {
        let unbonded_list = unbond_history()
            .idx
            .released
            .prefix(released.unwrap().to_string())
            .range(storage, None, None, Order::Ascending)
            .map(|n| n.unwrap().1)
            .collect::<Vec<_>>();

        return Ok(unbonded_list);
    }

    Ok(vec![])
}
