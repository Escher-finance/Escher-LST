use cosmwasm_std::{Addr, DepsMut, Env, Response};

use crate::ContractError;

pub fn update_ownership(
    deps: DepsMut,
    env: Env,
    new_owner: Addr,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    cw_ownable::update_ownership(deps, &env.block, &new_owner, action)?;
    Ok(Response::new())
}
