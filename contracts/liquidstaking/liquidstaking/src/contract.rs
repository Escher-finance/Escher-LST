use crate::instantiate::create_reward;
use crate::token_factory_api::TokenFactoryMsg;
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    CosmosMsg, Decimal, DepsMut, DistributionMsg, Env, MessageInfo, Order, Response, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::execute;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::state::{
    unbond_record, Balance, Parameters, State, Validator, ValidatorsRegistry, BALANCE, LOG,
    PARAMETERS, QUOTE_TOKEN, STATE, VALIDATORS_REGISTRY,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:evm_union_liquid_staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
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

    // create reward contract message to instantiate reward contract that will receive staking reward
    let (reward_msg, reward_addr) = create_reward(
        &deps,
        &env,
        msg.salt,
        msg.reward_code_id,
        env.clone().contract.address,
        msg.fee_receiver.clone(),
        msg.fee_rate.clone(),
        msg.underlying_coin_denom.clone(),
    )?;

    let params = Parameters {
        underlying_coin_denom: msg.underlying_coin_denom,
        liquidstaking_denom: msg.liquidstaking_denom,
        ucs03_relay_contract: msg.ucs03_relay_contract,
        unbonding_time: msg.unbonding_time,
        cw20_address: msg.cw20_address,
        reward_address: reward_addr.clone(),
        fee_rate: msg.fee_rate,
        fee_receiver: msg.fee_receiver,
    };
    PARAMETERS.save(deps.storage, &params)?;

    let state = State {
        exchange_rate: Decimal::one(),
        total_delegated_amount: Uint128::new(0),
        total_bond_amount: Uint128::new(0),
        total_supply: Uint128::new(0),
        bond_counter: 0,
        last_bond_time: 0,
    };
    STATE.save(deps.storage, &state)?;

    for quote_token in msg.quote_tokens {
        QUOTE_TOKEN.save(deps.storage, quote_token.channel_id, &quote_token)?;
    }

    let set_withdraw_msg: CosmosMsg<TokenFactoryMsg> =
        CosmosMsg::Distribution(DistributionMsg::SetWithdrawAddress {
            address: reward_addr.to_string(),
        });

    let msgs: Vec<CosmosMsg<TokenFactoryMsg>> = vec![reward_msg, set_withdraw_msg];

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_messages(msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    match msg {
        ExecuteMsg::Bond {
            staker,
            amount,
            salt,
        } => execute::bond(deps, env, info, staker, amount, salt),
        ExecuteMsg::Unbond { staker, amount } => execute::unbond(deps, env, info, staker, amount),
        ExecuteMsg::SetTokenAdmin { denom, new_admin } => {
            execute::set_token_admin(deps, info, denom, new_admin)
        }
        ExecuteMsg::ProcessRewards {} => execute::process_rewards(deps, env, info),
        ExecuteMsg::ProcessUnbonding { id, salt } => {
            execute::process_unbonding(deps, env, info, id, salt)
        }
        ExecuteMsg::Reset {} => execute::reset(deps, env, info),
        ExecuteMsg::UpdateOwnership(action) => execute::update_ownership(deps, env, info, action),
        ExecuteMsg::UpdateValidators { validators } => {
            execute::update_validators(deps, env, info, validators)
        }
        ExecuteMsg::SetParameters {
            underlying_coin_denom,
            liquidstaking_denom,
            ucs03_relay_contract,
            unbonding_time,
            cw20_address,
            reward_address,
            fee_receiver,
            fee_rate,
        } => execute::set_parameters(
            deps,
            env,
            info,
            underlying_coin_denom,
            liquidstaking_denom,
            ucs03_relay_contract,
            unbonding_time,
            cw20_address,
            reward_address,
            fee_receiver,
            fee_rate,
        ),
        ExecuteMsg::UpdateQuoteToken {
            channel_id,
            quote_token,
        } => execute::update_quote_token(deps, env, info, channel_id, quote_token),
        ExecuteMsg::Redelegate {} => execute::redelegate(deps, env, info),
        ExecuteMsg::MoveToReward {} => execute::move_to_reward(deps, env, info),
        ExecuteMsg::Transfer {
            amount,
            base_denom,
            receiver,
            ucs03_channel_id,
            ucs03_relay_contract,
            quote_token,
            salt,
        } => execute::transfer(
            deps,
            env,
            info,
            amount,
            base_denom,
            receiver,
            ucs03_channel_id,
            ucs03_relay_contract,
            quote_token,
            salt,
        ),
        ExecuteMsg::TransferToOwner {} => execute::transfer_to_owner(deps, env, info),
        ExecuteMsg::OnZkgm {
            channel_id,
            sender,
            message,
        } => execute::on_zkgm(deps, env, info, channel_id, sender, message),
        ExecuteMsg::MigrateReward { code_id } => execute::migrate_reward(deps, env, info, code_id),
        ExecuteMsg::TransferReward {} => execute::transfer_reward(deps),
        // ExecuteMsg::Burn { amount } => execute::burn(deps, env, info, amount),
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

    let mut unbonded_list = unbond_record()
        .idx
        .released
        .prefix("false".to_string())
        .range(deps.storage, None, None, Order::Descending)
        .map(|n| n.unwrap().1)
        .collect::<Vec<_>>();

    for unbonded in unbonded_list.iter_mut() {
        if unbonded.released_height > 0 {
            unbonded.released = true;
            unbond_record().save(deps.storage, unbonded.id, &unbonded)?;
        }
    }

    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}
