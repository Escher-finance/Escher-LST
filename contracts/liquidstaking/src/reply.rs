use crate::error::ContractError;
use crate::state::{Parameters, BALANCE, PARAMETERS};
use cosmwasm_std::{entry_point, DepsMut, Env, Reply, Response};

pub const MINT_TOKENS_REPLY_ID: u64 = 123;

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if !msg.result.is_ok() {
        let err = msg.result.unwrap_err();
        return Err(ContractError::ReplyError {
            message: err.to_string(),
        });
    }

    match msg.id {
        MINT_TOKENS_REPLY_ID => return on_mint_tokens(deps, env, msg),
        _ => return Ok(Response::new()),
    };
}

fn on_mint_tokens(deps: DepsMut, env: Env, _msg: Reply) -> Result<Response, ContractError> {
    let params: Parameters = PARAMETERS.load(deps.storage)?;
    let lst_balance = deps
        .querier
        .query_balance(env.contract.address.to_string(), params.liquidstaking_denom)?;
    BALANCE.save(deps.storage, &lst_balance.amount)?;

    Ok(Response::new())
}
