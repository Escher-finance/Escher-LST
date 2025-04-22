use crate::instantiate::create_reward;
use crate::utils::batch::{batches, Batch};
use crate::utils::validation::{validate_quote_tokens, validate_validators};
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    CosmosMsg, Decimal, DepsMut, DistributionMsg, Env, MessageInfo, Response, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::execute;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::state::{
    Parameters, State, Status, ValidatorsRegistry, WithdrawReward, PARAMETERS, PENDING_BATCH_ID,
    QUOTE_TOKEN, REWARD_BALANCE, SPLIT_REWARD_QUEUE, STATE, STATUS, VALIDATORS_REGISTRY,
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

    REWARD_BALANCE.save(deps.storage, &Uint128::new(0))?;

    validate_validators(&msg.validators)?;

    let reg = ValidatorsRegistry {
        validators: msg.validators,
    };

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
        batch_period: msg.batch_period,
        min_bond: msg.min_bond,
        min_unbond: msg.min_unbond,
        batch_limit: msg.batch_limit,
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

    validate_quote_tokens(&msg.quote_tokens)?;

    for quote_token in msg.quote_tokens {
        QUOTE_TOKEN.save(deps.storage, quote_token.channel_id, &quote_token)?;
    }

    let set_withdraw_msg: CosmosMsg =
        CosmosMsg::Distribution(DistributionMsg::SetWithdrawAddress {
            address: reward_addr.to_string(),
        });

    let msgs: Vec<CosmosMsg> = vec![reward_msg, set_withdraw_msg];

    SPLIT_REWARD_QUEUE.save(
        deps.storage,
        &WithdrawReward {
            target_amount: Uint128::zero(),
            withdrawed_amount: Uint128::zero(),
        },
    )?;

    let pending_batch = Batch::new(
        1,
        Uint128::zero(),
        env.block.time.seconds() + params.batch_period,
    );
    batches().save(deps.storage, pending_batch.id, &pending_batch)?;
    PENDING_BATCH_ID.save(deps.storage, &pending_batch.id)?;

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
        ExecuteMsg::Bond { slippage, expected } => {
            execute::bond(deps, env, info, slippage, expected)
        }
        ExecuteMsg::Receive(cw20_msg) => execute::receive(deps, env, info, cw20_msg),
        ExecuteMsg::SubmitBatch {} => execute::submit_batch(deps, env, info),
        ExecuteMsg::ProcessRewards {} => execute::process_rewards(deps, env, info),
        ExecuteMsg::ProcessBatchWithdrawal { id, salt } => {
            execute::process_batch_withdrawal(deps, env, info, id, salt)
        }
        ExecuteMsg::SetBatchReceivedAmount { id, amount } => {
            execute::set_batch_received_amount(deps, env, info, id, amount)
        }
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
            batch_period,
            min_bond,
            min_unbond,
            batch_limit,
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
            batch_period,
            min_bond,
            min_unbond,
            batch_limit,
        ),
        ExecuteMsg::UpdateQuoteToken {
            channel_id,
            quote_token,
        } => execute::update_quote_token(deps, env, info, channel_id, quote_token),
        ExecuteMsg::Redelegate {} => execute::redelegate(deps, env, info),
        ExecuteMsg::SetExecutor { executor } => execute::set_executor(deps, info, executor),
        ExecuteMsg::OnZkgm {
            caller: _caller,
            path: _path,
            source_channel_id: _source_channel_id,
            destination_channel_id,
            sender,
            message,
            relayer: _relayer,
            relayer_msg: _relayer_msg,
        } => execute::on_zkgm(deps, env, info, destination_channel_id, sender, message),
        ExecuteMsg::MigrateReward { code_id } => execute::migrate_reward(deps, env, info, code_id),
        ExecuteMsg::SetStatus(new_status) => execute::set_status(deps, info, new_status),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATUS.save(
        deps.storage,
        &Status {
            bond_is_paused: false,
            unbond_is_paused: false,
        },
    )?;
    Ok(Response::default())
}
