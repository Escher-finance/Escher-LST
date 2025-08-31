use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    entry_point, from_json, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult,
};
use cw2::set_contract_version;
use depolama::StorageExt;
use on_zkgm_call_proxy::OnProxyOnZkgmCall;
use semver::Version;
use unionlabs_primitives::U256;

use crate::{
    error::ContractError,
    execute::{
        accept_ownership, bond, circuit_breaker, receive_rewards, receive_unstaked_tokens,
        resume_contract, revoke_ownership_transfer, submit_batch, transfer_ownership, unbond,
        withdraw,
    },
    helpers::query_and_validate_unbonding_period,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg, RemoteExecuteMsg},
    query::{
        query_all_unstake_requests, query_batch, query_batches, query_batches_by_ids, query_config,
        query_pending_batch, query_state, query_unstake_requests,
    },
    state::{
        AccountingStateStore, Admin, Batches, ConfigStore, LstAddress, Monitors, OnZkgmCallProxy,
        PendingBatchId, ProtocolFeeConfigStore, StakerAddress, Zkgm,
    },
    types::{AccountingState, Batch, BatchId, Config, Staker, MAX_FEE_RATE},
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
    let InstantiateMsg {
        native_token_denom,
        minimum_liquid_stake_amount,
        staker_address,
        protocol_fee_config,
        lst_address,
        batch_period_seconds,
        monitors,
        admin,
        ucs03_zkgm_address,
        on_zkgm_call_proxy_address,
    } = msg;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    query_and_validate_unbonding_period(deps.as_ref(), batch_period_seconds)?;

    if protocol_fee_config.fee_rate > MAX_FEE_RATE {
        return Err(ContractError::InvalidProtocolFeeRate);
    }

    // save various addresses
    deps.storage.write_item::<Admin>(&admin);
    deps.storage.write_item::<StakerAddress>(&staker_address);
    deps.storage.write_item::<LstAddress>(&lst_address);
    deps.storage.write_item::<Zkgm>(&ucs03_zkgm_address);
    deps.storage
        .write_item::<OnZkgmCallProxy>(&on_zkgm_call_proxy_address);
    for monitor in monitors {
        deps.storage.write::<Monitors>(&monitor, &());
    }

    // save configs
    deps.storage
        .write_item::<ProtocolFeeConfigStore>(&protocol_fee_config);
    deps.storage.write_item::<ConfigStore>(&Config {
        native_token_denom,
        minimum_liquid_stake_amount: minimum_liquid_stake_amount.into(),
        batch_period_seconds,
    });
    deps.storage
        .write_item::<AccountingStateStore>(&AccountingState {
            total_bonded_native_tokens: 0,
            total_issued_lst: 0,
            total_reward_amount: 0,
        });

    // init first batch
    deps.storage.write::<Batches>(
        &BatchId::ONE,
        &Batch::new_pending(env.block.time.seconds() + batch_period_seconds),
    );
    deps.storage.write_item::<PendingBatchId>(&BatchId::ONE);

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", admin))
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
    match msg {
        ExecuteMsg::Bond {
            mint_to,
            min_mint_amount,
        } => bond(deps, info, mint_to, None, min_mint_amount.u128()),
        ExecuteMsg::Unbond { amount, staker } => unbond(
            deps,
            env,
            info,
            amount.u128(),
            Staker::Local {
                address: staker.to_string(),
            },
        ),
        ExecuteMsg::SubmitBatch {} => submit_batch(deps, env),
        ExecuteMsg::Withdraw {
            batch_id,
            staker,
            withdraw_to_address,
        } => withdraw(
            deps,
            info,
            batch_id,
            Staker::Local {
                address: staker.to_string(),
            },
            withdraw_to_address,
        ),
        ExecuteMsg::TransferOwnership { new_owner } => {
            transfer_ownership(deps, env, info, new_owner)
        }
        ExecuteMsg::AcceptOwnership {} => accept_ownership(deps, env, info),
        ExecuteMsg::RevokeOwnershipTransfer {} => revoke_ownership_transfer(deps, env, info),
        ExecuteMsg::ReceiveRewards {} => receive_rewards(deps, info),
        ExecuteMsg::ReceiveUnstakedTokens { batch_id } => {
            receive_unstaked_tokens(deps, env, info, batch_id)
        }
        ExecuteMsg::CircuitBreaker {} => circuit_breaker(deps, env, info),
        ExecuteMsg::ResumeContract {
            total_bonded_native_tokens,
            total_liquid_stake_token,
            total_reward_amount,
        } => {
            // resume_contract(
            //     deps,
            //     env,
            //     info,
            //     total_bonded_native_tokens,
            //     total_liquid_stake_token,
            //     total_reward_amount,
            // );
            todo!()
        }
        ExecuteMsg::SlashBatches { new_amounts } => {
            // slash_batches(deps, info, new_amounts);
            todo!()
        }
        ExecuteMsg::OnProxyOnZkgmCall(OnProxyOnZkgmCall { on_zkgm_msg, msg }) => {
            // TODO: ASSERT CALLER

            let msg = from_json::<RemoteExecuteMsg>(msg)?;

            // this is Call.message as sent from the source chain
            match msg {
                RemoteExecuteMsg::Bond {
                    mint_to,
                    min_mint_amount,
                } => bond(
                    deps,
                    info,
                    mint_to,
                    Some(on_zkgm_msg.relayer),
                    min_mint_amount.u128(),
                ),
                RemoteExecuteMsg::Unbond { amount } => unbond(
                    deps,
                    env,
                    info,
                    amount.u128(),
                    Staker::Remote {
                        // REVIEW: WHat address to use here?
                        address: on_zkgm_msg.sender,
                        channel_id: on_zkgm_msg.destination_channel_id,
                        path: U256::from_be_bytes(on_zkgm_msg.path.to_be_bytes()),
                    },
                ),
                RemoteExecuteMsg::Withdraw {
                    batch_id,
                    withdraw_to_address,
                } => withdraw(
                    deps,
                    info,
                    batch_id,
                    Staker::Remote {
                        address: on_zkgm_msg.sender,
                        channel_id: on_zkgm_msg.destination_channel_id,
                        path: U256::from_be_bytes(on_zkgm_msg.path.to_be_bytes()),
                    },
                    withdraw_to_address,
                ),
            }
        }
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
        QueryMsg::UnstakeRequests { user } => to_json_binary(&query_unstake_requests(deps, user)?),
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

    Ok(Response::new())
}
