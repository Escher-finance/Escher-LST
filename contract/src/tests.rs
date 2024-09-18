use crate::contract::instantiate;
use crate::msg::{InstantiateMsg, QueryMsg};
use crate::query::query;
use crate::state::Config;
use crate::ContractError;
use cosmwasm_std::testing::{message_info, mock_dependencies_with_balance, mock_env, MockApi};
use cosmwasm_std::{coins, from_json, Addr, DepsMut, Env, Response};

fn set_up(deps: DepsMut, env: Env, validators: Vec<Addr>) -> Result<Response, ContractError> {
    let denom_name: String = "muno".to_string();
    let msg = InstantiateMsg {
        underlying_coin_denom: denom_name.clone(),
        validators,
    };

    let creator = MockApi::default().addr_make("owner");
    let info = message_info(&creator, &coins(2, denom_name.as_str()));
    let res = instantiate(deps, env, info, msg).unwrap();
    Ok(res)
}

#[test]
fn proper_instantiation() {
    let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

    let validator = deps.api.addr_make("validator");
    let env = mock_env();
    let res = set_up(deps.as_mut(), env, vec![validator]);
    assert_eq!(res.unwrap().messages.len(), 0);
}

#[test]
fn initial_query() {
    let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
    let validator = deps.api.addr_make("validator");
    let env = mock_env();
    let _ = set_up(deps.as_mut(), env.clone(), vec![validator.clone()]);

    let msg = QueryMsg::Config {};
    let config: Config = from_json(query(deps.as_ref(), env, msg).unwrap()).unwrap();
    assert_eq!(config.validators.first().unwrap(), validator);
}
