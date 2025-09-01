use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, Event, MessageInfo, Response, StdError, StdResult, entry_point,
    from_json, to_json_binary,
};
use cw2::set_contract_version;
use depolama::StorageExt;
use on_zkgm_call_proxy::OnProxyOnZkgmCall;
use semver::Version;
use serde::{Deserialize, Serialize};
use unionlabs_primitives::U256;

use crate::{
    error::ContractError,
    execute::{
        accept_ownership, bond, circuit_breaker, receive_rewards, receive_unstaked_tokens,
        resume_contract, slash_batches, submit_batch, transfer_ownership, unbond, update_config,
        withdraw,
    },
    helpers::query_and_validate_unbonding_period,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg, RemoteExecuteMsg},
    query::{
        query_all_unstake_requests, query_batch, query_batches, query_batches_by_ids, query_config,
        query_pending_batch, query_state, query_unstake_requests_by_staker_hash,
    },
    state::{
        AccountingStateStore, Admin, Batches, ConfigStore, LstAddress, Monitors, OnZkgmCallProxy,
        PendingBatchId, ProtocolFeeConfigStore, StakerAddress, Zkgm,
    },
    types::{AccountingState, Batch, BatchId, Config, MAX_FEE_RATE, Staker},
};

// Version information for migration
pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

///////////////////
/// INSTANTIATE ///
///////////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
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

    let unbonding_period =
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
    deps.storage
        .write_item::<Monitors>(&monitors.into_iter().map(Into::into).collect());

    // save configs and state
    deps.storage
        .write_item::<ProtocolFeeConfigStore>(&protocol_fee_config);
    deps.storage.write_item::<ConfigStore>(&Config {
        native_token_denom: native_token_denom.clone(),
        minimum_liquid_stake_amount,
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

    Ok(Response::new().add_event(
        Event::new("init")
            .add_attribute("admin", admin)
            .add_attribute("native_token_denom", native_token_denom)
            .add_attribute(
                "minimum_liquid_stake_amount",
                minimum_liquid_stake_amount.to_string(),
            )
            .add_attribute(
                "protocol_fee_rate",
                protocol_fee_config.fee_rate.to_string(),
            )
            .add_attribute("protocol_fee_recipient", protocol_fee_config.fee_recipient)
            .add_attribute("current_unbonding_period", unbonding_period.to_string())
            .add_attribute("staker_address", staker_address)
            .add_attribute("lst_address", lst_address)
            .add_attribute("ucs03_zkgm_address", ucs03_zkgm_address)
            .add_attribute("on_zkgm_call_proxy_address", on_zkgm_call_proxy_address),
    ))
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
        ExecuteMsg::RevokeOwnershipTransfer {} => {
            // revoke_ownership_transfer(deps, env, info)
            todo!()
        }
        ExecuteMsg::UpdateConfig {
            protocol_fee_config,
            monitors,
            batch_period,
        } => update_config(deps, info, protocol_fee_config, monitors, batch_period),
        ExecuteMsg::ReceiveRewards {} => receive_rewards(deps, info),
        ExecuteMsg::ReceiveUnstakedTokens { batch_id } => {
            receive_unstaked_tokens(deps, env, info, batch_id)
        }
        ExecuteMsg::CircuitBreaker {} => circuit_breaker(deps, info),
        ExecuteMsg::ResumeContract {
            total_bonded_native_tokens,
            total_issued_lst,
            total_reward_amount,
        } => resume_contract(
            deps,
            info,
            AccountingState {
                total_bonded_native_tokens: total_bonded_native_tokens.u128(),
                total_issued_lst: total_issued_lst.u128(),
                total_reward_amount: total_reward_amount.u128(),
            },
        ),
        ExecuteMsg::SlashBatches { new_amounts } => slash_batches(deps, info, new_amounts),
        ExecuteMsg::OnProxyOnZkgmCall(OnProxyOnZkgmCall { on_zkgm_msg, msg }) => {
            if deps.storage.read_item::<OnZkgmCallProxy>()? != info.sender {
                return Err(ContractError::Unauthorized {
                    sender: info.sender.clone(),
                });
            }

            // this is `Call.message` as sent from the source chain
            match from_json::<RemoteExecuteMsg>(msg)? {
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
                        // REVIEW: What address to use here?
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
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::AccountingState {} => to_json_binary(&query_state(deps)?),
        QueryMsg::Batch { batch_id } => to_json_binary(&query_batch(deps, batch_id)?),
        QueryMsg::Batches {
            start_after,
            limit,
            status,
        } => to_json_binary(&query_batches(deps, start_after, limit, status)?),
        QueryMsg::BatchesByIds { batch_ids } => {
            to_json_binary(&query_batches_by_ids(deps, &batch_ids)?)
        }
        QueryMsg::PendingBatch {} => to_json_binary(&query_pending_batch(deps)?),
        QueryMsg::UnstakeRequestsByStaker { staker } => {
            to_json_binary(&query_unstake_requests_by_staker_hash(deps, staker.hash())?)
        }
        QueryMsg::UnstakeRequestsByStakerHash { staker_hash } => {
            to_json_binary(&query_unstake_requests_by_staker_hash(deps, staker_hash)?)
        }
        QueryMsg::AllUnstakeRequests { start_after, limit } => {
            to_json_binary(&query_all_unstake_requests(deps, start_after, limit)?)
        }
    }
}

///////////////
/// MIGRATE ///
///////////////

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
