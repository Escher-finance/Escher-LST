use crate::token_factory_api::TokenFactoryMsg;
use cosmwasm_std::entry_point;
use cosmwasm_std::{Decimal, DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::execute;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::state::{
    Balance, Parameters, State, Validator, ValidatorsRegistry, BALANCE, LOG, PARAMETERS, STATE,
    VALIDATORS_REGISTRY,
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

    let binding = info.sender.to_string();
    let owner = Some(binding.as_ref());
    cw_ownable::initialize_owner(deps.storage, deps.api, owner)?;

    LOG.save(deps.storage, &"".into())?;

    let balance = Balance {
        amount: Uint128::new(0),
        last_updated: 0,
    };
    BALANCE.save(deps.storage, &balance)?;

    let mut validators: Vec<Validator> = vec![];
    for validator in msg.validators {
        validators.push({
            Validator {
                address: validator.address,
                weight: validator.weight,
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
        unbonding_time: msg.unbonding_time,
        cw20_address: msg.cw20_address,
        reward_address: None,
    };
    PARAMETERS.save(deps.storage, &params)?;

    let chain;
    if cfg!(nonunion) {
        chain = "nonunion".into();
    } else {
        chain = "union".into();
    }

    let state = State {
        exchange_rate: Decimal::one(),
        total_delegated_amount: Uint128::new(0),
        total_bond_amount: Uint128::new(0),
        total_lst_supply: Uint128::new(0),
        bond_counter: 0,
        last_bond_time: 0,
        chain,
    };
    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    match msg {
        ExecuteMsg::Bond { staker, amount } => execute::bond(deps, env, info, staker, amount),
        ExecuteMsg::Unbond { staker, amount } => execute::unbond(deps, env, info, staker, amount),
        ExecuteMsg::Transfer { amount, receiver } => {
            execute::transfer(deps, env, info, amount, receiver)
        }
        ExecuteMsg::SetTokenAdmin { denom, new_admin } => {
            execute::set_token_admin(deps, info, denom, new_admin)
        }
        ExecuteMsg::ProcessRewards {} => execute::process_rewards(deps, env, info),
        ExecuteMsg::ProcessUnbonding { id } => execute::process_unbonding(deps, env, info, id),
        ExecuteMsg::Reset {} => execute::reset(deps, env, info),
        ExecuteMsg::UpdateOwnership(action) => execute::update_ownership(deps, env, info, action),
        ExecuteMsg::UpdateValidators { validators } => {
            execute::update_validators(deps, env, info, validators)
        }
        ExecuteMsg::SetParameters {
            underlying_coin_denom,
            liquidstaking_denom,
            ucs01_channel,
            ucs01_relay_contract,
            unbonding_time,
            cw20_address,
            reward_address,
        } => execute::set_parameters(
            deps,
            env,
            info,
            underlying_coin_denom,
            liquidstaking_denom,
            ucs01_channel,
            ucs01_relay_contract,
            unbonding_time,
            cw20_address,
            reward_address,
        ),
        ExecuteMsg::Redelegate {} => execute::redelegate(deps, env, info),
    }
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
