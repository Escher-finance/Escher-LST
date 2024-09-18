#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Decimal, Uint128};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{Config, Parameters, State, CONFIG, PARAMETERS, STATE};

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:evm-union-liquid-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        validators: msg.validators,
    };
    CONFIG.save(deps.storage, &config)?;

    let params = Parameters {
        underlying_coin_denom: msg.underlying_coin_denom,
    };
    PARAMETERS.save(deps.storage, &params)?;

    let state = State {
        exchange_rate: Decimal::one(),
        total_bond_amount: Uint128::new(0),
        last_unbonded_time: 0,
    };
    STATE.save(deps.storage, &state)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    unimplemented!()
}
