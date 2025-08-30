use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Uint128,
};
use cw2::set_contract_version;
use semver::Version;

use crate::{
    error::ContractError,
    execute::{
        circuit_breaker, execute_accept_ownership, execute_bond, execute_revoke_ownership_transfer,
        execute_submit_batch, execute_transfer_ownership, execute_unbond, execute_withdraw,
        receive_rewards, receive_unstaked_tokens, resume_contract, slash_batches,
    },
    helpers::query_and_validate_unbonding_period,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    query::{
        query_all_unstake_requests, query_batch, query_batches, query_batches_by_ids, query_config,
        query_pending_batch, query_state, query_unstake_requests,
    },
    state::{
        assert_not_migrating, Config, State, ADMIN, BATCHES, CONFIG, MIGRATING, PENDING_BATCH_ID,
        STATE,
    },
    types::{Batch, MAX_TREASURY_FEE},
};

// Version information for migration
pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

///////////////////
/// INSTANTIATE ///
///////////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if msg.protocol_fee_config.fee_rate > MAX_TREASURY_FEE {
        return Err(ContractError::InvalidDaoTreasuryFee {});
    }

    ADMIN.set(deps.branch(), Some(msg.admin.clone()))?;

    query_and_validate_unbonding_period(deps.as_ref(), msg.batch_period)?;

    let config = Config {
        protocol_fee_config: msg.protocol_fee_config,
        liquid_stake_token_address: msg.liquid_stake_token_address,
        monitors: msg.monitors,
        batch_period: msg.batch_period,
        stopped: true, // we start stopped
        native_token_denom: msg.native_token_denom,
        minimum_liquid_stake_amount: msg.minimum_liquid_stake_amount,
        ucs03_zkgm_address: msg.ucs03_zkgm_address,
        funded_dispatch_address: msg.funded_dispatch_address,
        staker_address: msg.staker_address,
    };

    // Init State
    let state = State {
        total_native_token: Uint128::zero(),
        total_bonded_lst: Uint128::zero(),
        pending_owner: None,
        total_reward_amount: Uint128::zero(),
        owner_transfer_min_time: None,
    };

    // Set pending batch and batches
    BATCHES.save(
        deps.storage,
        1,
        &Batch::new_pending(env.block.time.seconds() + config.batch_period),
    )?;
    PENDING_BATCH_ID.save(deps.storage, &1)?;
    CONFIG.save(deps.storage, &config)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", msg.admin))
}

///////////////
/// EXECUTE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    assert_not_migrating(deps.as_ref())?;

    match msg {
        ExecuteMsg::Bond {
            mint_to,
            recipient_channel_id,
            min_mint_amount,
        } => execute_bond(
            deps,
            env,
            info,
            mint_to,
            recipient_channel_id,
            min_mint_amount,
        ),
        ExecuteMsg::Unbond { amount, staker } => execute_unbond(deps, env, info, amount, staker),

        ExecuteMsg::SubmitBatch {} => execute_submit_batch(deps, env),

        ExecuteMsg::Withdraw { batch_id, staker } => execute_withdraw(deps, info, batch_id, staker),

        // ownership msgs
        ExecuteMsg::TransferOwnership { new_owner } => {
            execute_transfer_ownership(deps, env, info, new_owner)
        }
        ExecuteMsg::AcceptOwnership {} => execute_accept_ownership(deps, env, info),
        ExecuteMsg::RevokeOwnershipTransfer {} => {
            execute_revoke_ownership_transfer(deps, env, info)
        }

        // ExecuteMsg::UpdateConfig {
        //     native_chain_config,
        //     protocol_chain_config,
        //     protocol_fee_config,
        //     monitors,
        //     batch_period,
        // } => update_config(
        //     deps,
        //     env,
        //     info,
        //     native_chain_config,
        //     protocol_chain_config,
        //     protocol_fee_config,
        //     monitors,
        //     batch_period,
        // ),
        ExecuteMsg::ReceiveRewards {} => receive_rewards(deps, info),
        ExecuteMsg::ReceiveUnstakedTokens { batch_id } => {
            receive_unstaked_tokens(deps, env, info, batch_id)
        }

        // pause/resume msgs
        ExecuteMsg::CircuitBreaker {} => circuit_breaker(deps, env, info),
        ExecuteMsg::ResumeContract {
            total_native_token,
            total_liquid_stake_token,
            total_reward_amount,
        } => resume_contract(
            deps,
            env,
            info,
            total_native_token,
            total_liquid_stake_token,
            total_reward_amount,
        ),
        ExecuteMsg::SlashBatches { new_amounts } => slash_batches(deps, info, new_amounts),
    }
}

/////////////
/// QUERY ///
/////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::State {} => to_json_binary(&query_state(deps)?),
        QueryMsg::Batch { id } => to_json_binary(&query_batch(deps, id)?),
        QueryMsg::Batches {
            start_after,
            limit,
            status,
        } => to_json_binary(&query_batches(deps, start_after, limit, status)?),
        QueryMsg::BatchesByIds { ids } => to_json_binary(&query_batches_by_ids(deps, ids)?),
        QueryMsg::PendingBatch {} => to_json_binary(&query_pending_batch(deps)?),
        QueryMsg::UnstakeRequests { user } => {
            to_json_binary(&query_unstake_requests(deps, user.into_string())?)
        }
        QueryMsg::AllUnstakeRequests { start_after, limit } => {
            to_json_binary(&query_all_unstake_requests(deps, start_after, limit)?)
        }
    }
}

///////////////
/// MIGRATE ///
///////////////

#[cw_serde]
pub struct MigrateMsg {}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let current_version = cw2::get_contract_version(deps.storage)?;
    if CONTRACT_NAME != current_version.contract.as_str() {
        return Err(StdError::generic_err("Cannot upgrade to a different contract").into());
    }

    let version: Version = current_version
        .version
        .parse()
        .map_err(|_| StdError::generic_err("Invalid contract version"))?;
    let new_version: Version = CONTRACT_VERSION
        .parse()
        .map_err(|_| StdError::generic_err("Invalid contract version"))?;

    // Prevent downgrade
    if version > new_version {
        return Err(StdError::generic_err("Cannot upgrade to a previous contract version").into());
    }
    // if same version return
    if version == new_version {
        let is_migrating = MIGRATING.may_load(deps.storage)?.unwrap_or(false);
        if !is_migrating {
            return Err(StdError::generic_err("Cannot migrate to the same version.").into());
        }
    }

    Ok(Response::new())
}
