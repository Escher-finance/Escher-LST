use std::collections::{HashMap, HashSet};

use crate::instantiate::create_reward;
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    CosmosMsg, Decimal, DepsMut, DistributionMsg, Env, MessageInfo, Response, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::execute;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::state::{
    Balance, Parameters, State, ValidatorsRegistry, BALANCE, LOG, PARAMETERS, QUOTE_TOKEN, STATE,
    VALIDATORS_REGISTRY,
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
) -> Result<Response, ContractError> {
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

    let unique_validators_len = msg
        .validators
        .iter()
        .cloned()
        .map(|validator| validator.address)
        .collect::<HashSet<_>>()
        .len();

    if unique_validators_len != msg.validators.len() {
        return Err(ContractError::InvalidValidators {});
    }

    let validators = msg.validators.clone();

    for validator in &validators {
        deps.api.addr_validate(&validator.address)?;
        if validator.weight == 0 {
            return Err(ContractError::InvalidValidators {});
        }
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

    let unique_quote_tokens_len = msg
        .quote_tokens
        .iter()
        .cloned()
        .map(|quote_token| quote_token.channel_id)
        .collect::<HashSet<_>>()
        .len();

    if unique_quote_tokens_len != msg.quote_tokens.len() {
        return Err(ContractError::InvalidQuoteTokens {});
    }

    for quote_token in msg.quote_tokens {
        QUOTE_TOKEN.save(deps.storage, quote_token.channel_id, &quote_token)?;
    }

    let set_withdraw_msg: CosmosMsg =
        CosmosMsg::Distribution(DistributionMsg::SetWithdrawAddress {
            address: reward_addr.to_string(),
        });

    let msgs: Vec<CosmosMsg> = vec![reward_msg, set_withdraw_msg];

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
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Bond { amount, salt } => execute::bond(deps, env, info, amount, salt),
        ExecuteMsg::Unbond { amount } => execute::unbond(deps, env, info, amount),

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
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}
