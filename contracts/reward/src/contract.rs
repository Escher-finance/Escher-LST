#[cfg(not(feature = "library"))]
use cosmwasm_std::{entry_point, to_json_binary};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Storage};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::execute;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Config, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:rewards";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let binding = info.sender.to_string();
    let owner = Some(binding.as_ref());
    cw_ownable::initialize_owner(deps.storage, deps.api, owner)?;

    let config = Config {
        lst_contract_address: msg.lst_contract,
        fee_receiver: msg.fee_receiver,
        fee_rate: msg.fee_rate,
        coin_denom: msg.coin_denom,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SplitReward {} => execute::split_reward(deps, env, info),
        ExecuteMsg::SetConfig {
            lst_contract_address,
            fee_receiver,
            fee_rate,
            coin_denom,
        } => execute::set_config(
            deps,
            env,
            info,
            lst_contract_address,
            fee_receiver,
            fee_rate,
            coin_denom,
        ),
        ExecuteMsg::TransferToOwner {} => execute::transfer_to_owner(deps, env, info),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_get_config(deps.storage)?),
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}

pub fn query_get_config(storage: &dyn Storage) -> StdResult<Config> {
    let params = CONFIG.load(storage)?;
    Ok(params)
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let ver = cw2::get_contract_version(deps.storage)?;
    // ensure we are migrating from a compatible contract
    if ver.contract != CONTRACT_NAME {
        return Err(ContractError::InvalidContractName {});
    }
    let version: semver::Version = CONTRACT_VERSION.parse()?;
    let prev_version: semver::Version = ver.version.parse()?;
    if prev_version >= version {
        return Err(ContractError::InvalidMigrationVersion {
            expected: format!("> {prev_version}"),
            actual: CONTRACT_VERSION.to_string(),
        });
    }
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
