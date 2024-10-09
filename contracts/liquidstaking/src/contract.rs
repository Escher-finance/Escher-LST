use crate::token_factory_api::TokenFactoryMsg;
use cosmwasm_std::entry_point;
use cosmwasm_std::{Decimal, DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::execute;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::state::{
    Balance, Config, Parameters, State, Validator, ValidatorsRegistry, BALANCE, CONFIG, LOG,
    PARAMETERS, STATE, VALIDATORS_REGISTRY,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:evm_union_liquid_staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    LOG.save(deps.storage, &"".into())?;

    let config = Config {
        owner: info.sender.to_string(),
    };
    CONFIG.save(deps.storage, &config)?;

    let balance = Balance {
        amount: Uint128::new(0),
        last_updated: 0,
    };
    BALANCE.save(deps.storage, &balance)?;

    let mut validators: Vec<Validator> = vec![];
    for validator_addr in msg.validators {
        validators.push({
            Validator {
                address: validator_addr.to_string(),
            }
        })
    }

    let reg = ValidatorsRegistry { validators };
    VALIDATORS_REGISTRY.save(deps.storage, &reg)?;

    let params = Parameters {
        underlying_coin_denom: msg.underlying_coin_denom,
        liquidstaking_denom: msg.liquidstaking_denom,
        ucs01_channel: msg.ucs01_channel,
        ucs01_relay_contract: msg.ucs01_relay_contract,
    };
    PARAMETERS.save(deps.storage, &params)?;

    let state = State {
        exchange_rate: Decimal::one(),
        total_delegated_amount: Uint128::new(0),
        total_bond_amount: Uint128::new(0),
        total_lst_supply: Uint128::new(0),
        bond_counter: 0,
        last_bond_time: 0,
    };
    STATE.save(deps.storage, &state)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    match msg {
        ExecuteMsg::Bond { staker } => execute::bond(deps, env, info, staker),
        ExecuteMsg::Unbond { staker } => execute::unbond(deps, env, info, staker),
        ExecuteMsg::Transfer { amount, receiver } => {
            execute::transfer(deps, env, info, amount, receiver)
        }
        ExecuteMsg::SetOwner { new_owner } => execute::set_owner(deps, info, new_owner),
        ExecuteMsg::SetTokenAdmin { denom, new_admin } => {
            execute::set_token_admin(deps, info, denom, new_admin)
        }
        ExecuteMsg::BondRewards {} => execute::bond_rewards(deps, env, info),
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let ver = cw2::get_contract_version(deps.storage)?;
    // ensure we are migrating from a compatible contract
    if ver.contract != CONTRACT_NAME {
        return Err(ContractError::InvalidContractName {});
    }
    // note: it's better to do a proper semver comparison, but a string comparison *usually* works    #[allow(clippy::cmp_owned)]
    if ver.version >= CONTRACT_VERSION.to_string() {
        return Err(ContractError::InvalidContractVersion {
            message: "Current version is equal or newer than the new contract code".to_string(),
        });
    } // set the new version
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
