use cosmwasm_std::{
    attr, ensure, wasm_execute, Addr, BankMsg, Coin, Deps, DepsMut, Env, Event, MessageInfo,
    Response, Uint128,
};
use cw20::Cw20ExecuteMsg;
use cw_utils::must_pay;
use depolama::StorageExt;

use crate::{
    error::{ContractError, ContractResult},
    helpers::{compute_mint_amount, compute_unbond_amount, query_and_validate_unbonding_period},
    state::{
        AccountingStateStore, Admin, Batches, ConfigStore, LstAddress, Monitors, PendingBatchId,
        PendingOwnerStore, ProtocolFeeConfigStore, StakerAddress, Stopped, UnstakeRequests,
        UnstakeRequestsByStakerHash,
    },
    types::{
        AccountingState, Batch, BatchExpectedAmount, BatchId, BatchState, Config, PendingOwner,
        ProtocolFeeConfig, Staker, UnstakeRequest, UnstakeRequestKey,
    },
};

const FEE_RATE_DENOMINATOR: u64 = 100_000;
/// 7 days
const OWNERSHIP_CLAIM_DELAY_PERIOD: u64 = 60 * 60 * 24 * 7;

pub fn ensure_stopped(deps: Deps) -> Result<(), ContractError> {
    if deps.storage.read_item::<Stopped>()? {
        return Err(ContractError::NotStopped);
    }
    Ok(())
}

pub fn ensure_not_stopped(deps: Deps) -> Result<(), ContractError> {
    if deps.storage.read_item::<Stopped>()? {
        return Err(ContractError::Stopped);
    }
    Ok(())
}

fn ensure_admin(deps: Deps, info: &MessageInfo) -> Result<(), ContractError> {
    if deps.storage.read_item::<Admin>()? != info.sender {
        Err(ContractError::Unauthorized {
            sender: info.sender.clone(),
        })
    } else {
        Ok(())
    }
}

// TODO: Build out an allowances system?
pub fn ensure_sender(info: &MessageInfo, local_staker_address: &str) -> Result<(), ContractError> {
    if info.sender.as_str() != local_staker_address {
        Err(ContractError::Unauthorized {
            sender: info.sender.clone(),
        })
    } else {
        Ok(())
    }
}

/// 1. Ensure native tokens are provided.
/// 2. Ensure stake amount is >= minimum stake amount.
/// 3. Ensure minted LST amount is within slippage.
/// 4. Send funds to staker contract.
/// 5. Update state
pub fn bond(
    deps: DepsMut,
    info: MessageInfo,
    mint_to_address: Addr,
    // if set, this address will be sent the slippage (if any)
    relayer: Option<Addr>,
    min_mint_amount: u128,
) -> ContractResult<Response> {
    ensure_not_stopped(deps.as_ref())?;

    let config = deps.storage.read_item::<ConfigStore>()?;

    let amount = must_pay(&info, &config.native_token_denom)?.u128();

    ensure!(
        amount >= config.minimum_liquid_stake_amount,
        ContractError::MinimumLiquidStakeAmount {
            minimum_stake_amount: config.minimum_liquid_stake_amount,
            sent_amount: amount,
        }
    );

    let mut accounting_state = deps.storage.read_item::<AccountingStateStore>()?;

    // Compute mint amount
    let mint_amount = compute_mint_amount(
        accounting_state.total_bonded_native_tokens,
        accounting_state.total_issued_lst,
        amount,
    );

    // If mint amount is zero it is likely there was a an issue with rounding, return error and do not mint
    if mint_amount == 0 {
        return Err(ContractError::ComputedMintAmountIsZero);
    }

    let (mint_amount, slippage_and_relayer) =
        match (mint_amount.checked_sub(min_mint_amount), relayer) {
            (Some(0), _) | (Some(_), None) => {
                // either no slippage, (i.e. the exact amount was met) or there is no relayer to send the slippage to
                (mint_amount, None)
            }
            (Some(slippage), Some(relayer)) => {
                // mint slippage to the relayer as a fee
                (min_mint_amount, Some((slippage, relayer)))
            }
            (None, _) => {
                // slippage not met
                return Err(ContractError::MintAmountMismatch {
                    expected: min_mint_amount,
                    actual: mint_amount,
                });
            }
        };

    // transfer native token to multisig address
    // NOTE: In the original milkyway satking contracts, this was an ibc transfer message back to the multisig on the source chain since the liquid staking is *initiated* on the protocol chain (i.e. where this contract is deployed), but the *staking* happens on the native chain
    // TODO: This should be a wasm_execute call with a specific ExecuteMsg for the staker contract
    // TODO: Define the ExecuteMsg for the staker contract
    let transfer_funds_to_cw_account_msig_message = BankMsg::Send {
        to_address: deps.storage.read_item::<StakerAddress>()?.to_string(),
        amount: info.funds,
    };
    accounting_state.total_bonded_native_tokens += amount;
    accounting_state.total_issued_lst += mint_amount;
    deps.storage
        .write_item::<AccountingStateStore>(&accounting_state);

    let lst_address = deps.storage.read_item::<LstAddress>()?;

    let response = Response::new()
        .add_message(transfer_funds_to_cw_account_msig_message)
        // send the minted lst tokens to recipient
        .add_message(wasm_execute(
            // eU address
            &lst_address,
            &Cw20ExecuteMsg::Mint {
                amount: min_mint_amount.into(),
                recipient: mint_to_address.to_string(),
            },
            vec![],
        )?)
        // mint the slippage (if any), into the relayer (if any)
        .add_messages(
            slippage_and_relayer
                .clone()
                .map(|(slippage, relayer)| {
                    wasm_execute(
                        // eU address
                        lst_address,
                        &Cw20ExecuteMsg::Mint {
                            amount: slippage.into(),
                            recipient: relayer.to_string(),
                        },
                        vec![],
                    )
                })
                .transpose()?,
        )
        .add_event(
            Event::new("bond")
                .add_attribute("mint_to_address", mint_to_address.to_string())
                // NOTE: In practice, this will always be the funded-dispatch contract. This may need to be changed to emit the original sender from the source chain (if it exists).
                .add_attribute("sender", info.sender.to_string())
                .add_attribute("in_amount", amount.to_string())
                .add_attribute("mint_amount", mint_amount.to_string()),
        )
        .add_events(slippage_and_relayer.map(|(slippage, relayer)| {
            Event::new("bond_slippage_paid").add_attributes([
                attr("slippage", slippage.to_string()),
                attr("relayer", relayer),
            ])
        }));

    Ok(response)
}

/// Unbond the LST.
///
/// The LST is sent to this contract, and an unstaking request is added to the current batch. Once the batch is submitted, [`execute_withdraw`] can be called to withdraw the unstaked native token.
///
/// 1. Write the new unbond request to storage.
/// 2. Update the batch.
/// 3. Transfer the LST to this contract. Note that this requires an allowance to spend these tokens on behalf of the staker.
///
/// # LST Balance Tracking
///
/// It should be noted that this contract does NOT track the balance of the LST. The LST contract itself is expected to correctly track and maintain it's own balances. This prevents unbonding more tokens than there are in total, since the TransferFrom call to the LST will fail.
pub fn unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: u128,
    staker: Staker,
) -> ContractResult<Response> {
    ensure_not_stopped(deps.as_ref())?;

    // if the staker is local, then we need to ensure that the address allowed to unbond for the staker
    if let Staker::Local { address } = &staker {
        ensure_sender(&info, address)?;
    };

    let pending_batch_id = deps.storage.read_item::<PendingBatchId>()?;

    let staker_hash = staker.hash();

    // 1.
    let mut is_new_request = false;
    let unstake_request_key = UnstakeRequestKey {
        batch_id: pending_batch_id,
        staker_hash,
    };
    let updated_unstake_request = deps.storage.upsert::<UnstakeRequests, ContractError>(
        &unstake_request_key,
        |maybe_unstake_request| {
            Ok(match maybe_unstake_request {
                Some(r) => UnstakeRequest {
                    batch_id: r.batch_id,
                    staker: r.staker,
                    amount: r.amount + amount,
                },
                None => {
                    // this is a bit of a hack since .upsert() doesn't allow for returning anything else other than the T of the IndexMap. ideally it would return (R, T), write T to storage, and then return (R, T)
                    is_new_request = true;
                    UnstakeRequest {
                        batch_id: pending_batch_id,
                        staker: staker.clone(),
                        amount,
                    }
                }
            })
        },
    )?;

    deps.storage
        .write::<UnstakeRequestsByStakerHash>(&unstake_request_key, &updated_unstake_request);

    // 2.
    deps.storage
        .upsert::<Batches, ContractError>(&pending_batch_id, |batch| {
            let mut batch = batch.unwrap();
            batch.total_lst_to_burn += amount;
            if is_new_request {
                batch.unstake_requests_count += 1;
            }
            Ok(batch)
        })?;

    // 3.
    let lst_transfer_from_msg = wasm_execute(
        &deps.storage.read_item::<LstAddress>()?,
        &Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.to_string(),
            recipient: env.contract.address.to_string(),
            amount: amount.into(),
        },
        vec![],
    )?;

    // Batch [ Send, Call(IncAllow), Call(Unbond) ]

    let response = Response::new()
        .add_message(lst_transfer_from_msg)
        .add_event(
            Event::new("unbond")
                .add_attribute("staker_hash", staker_hash.to_string())
                .add_attribute("batch", pending_batch_id.to_string())
                .add_attribute("amount", amount.to_string())
                .add_attribute("is_new_request", is_new_request.to_string()),
        );

    Ok(match staker {
        Staker::Local { address } => response.add_attributes([
            attr("staker_type", "local"),
            attr("staker_address", address),
        ]),
        Staker::Remote {
            address,
            channel_id,
            path,
        } => response.add_attributes([
            attr("staker_type", "remote"),
            attr("staker_address", address.to_string()),
            attr("staker_channel_id", channel_id.to_string()),
            attr("staker_path", path.to_string()),
        ]),
    })
}

/// Submit batch and transition pending batch to submitted.
///
/// # Changes from original implementation
///
/// - oracle functionality was removed
///
/// TODO: Withdraw unstaked tokens in this function
pub fn submit_batch(deps: DepsMut, env: Env) -> ContractResult<Response> {
    ensure_not_stopped(deps.as_ref())?;

    let config = deps.storage.read_item::<ConfigStore>()?;

    let pending_batch_id = deps.storage.read_item::<PendingBatchId>()?;

    let batch = deps.storage.read::<Batches>(&pending_batch_id)?;

    let BatchState::Pending { submit_time } = batch.state else {
        return Err(ContractError::BatchAlreadySubmitted {
            batch_id: pending_batch_id,
        });
    };

    ensure!(
        env.block.time.seconds() >= submit_time,
        ContractError::BatchNotReady {
            now: env.block.time.seconds(),
            ready_at: submit_time,
        }
    );

    ensure!(
        batch.unstake_requests_count > 0,
        ContractError::BatchEmpty {
            batch_id: pending_batch_id
        }
    );

    let accounting_state = deps.storage.read_item::<AccountingStateStore>()?;

    // reduce underlying LST balance by batch total
    // ensure we have more issued LST than LST we're trying to burn
    let new_total_issued_lst = match accounting_state
        .total_issued_lst
        .checked_sub(batch.total_lst_to_burn)
    {
        Some(new_total_issued_lst) => new_total_issued_lst,
        None => {
            return Err(ContractError::UnbondSlippageExceeded {
                total_issued_lst: accounting_state.total_issued_lst,
                amount_to_unstake: batch.total_lst_to_burn,
            })
        }
    };

    let unbond_amount = compute_unbond_amount(
        accounting_state.total_bonded_native_tokens,
        accounting_state.total_issued_lst,
        batch.total_lst_to_burn,
    );

    deps.storage
        .write_item::<AccountingStateStore>(&AccountingState {
            // reduce underlying native token balance by unbonded amount
            total_bonded_native_tokens: accounting_state
                .total_bonded_native_tokens
                .checked_sub(unbond_amount)
                .ok_or_else(|| ContractError::AttemptedToUnbondMoreThanBonded {
                    unbond_amount,
                    total_bonded_native_tokens: accounting_state.total_bonded_native_tokens,
                })?,
            total_issued_lst: new_total_issued_lst,
            ..accounting_state
        });

    let unbonding_period =
        query_and_validate_unbonding_period(deps.as_ref(), config.batch_period_seconds)?;

    // save previously pending batch as submitted
    deps.storage.write::<Batches>(
        &pending_batch_id,
        &Batch {
            state: BatchState::Submitted {
                receive_time: env.block.time.seconds() + unbonding_period,
                expected_native_unstaked: unbond_amount,
            },
            ..batch
        },
    );

    // finally, save new pending batch
    let new_pending_batch_id = pending_batch_id.increment();
    deps.storage
        .write_item::<PendingBatchId>(&new_pending_batch_id);
    deps.storage.write::<Batches>(
        &new_pending_batch_id,
        &Batch::new_pending(env.block.time.seconds() + config.batch_period_seconds),
    );

    Ok(Response::new()
        // burn all unbonded LST tokens on batch submission
        .add_message(wasm_execute(
            deps.storage.read_item::<LstAddress>()?,
            &Cw20ExecuteMsg::Burn {
                amount: batch.total_lst_to_burn.into(),
            },
            vec![],
        )?)
        .add_event(
            Event::new("submit_batch")
                .add_attribute("batch_id", pending_batch_id.to_string())
                .add_attribute("batch_total", batch.total_lst_to_burn.to_string())
                .add_attribute("expected_unstaked", unbond_amount.to_string())
                .add_attribute("current_unbonding_period", unbonding_period.to_string()),
        ))
}

pub fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    batch_id: BatchId,
    staker: Staker,
    withdraw_to_address: Addr,
) -> ContractResult<Response> {
    let config = deps.storage.read_item::<ConfigStore>()?;

    ensure_not_stopped(deps.as_ref())?;

    // if the staker is local, then we need to ensure that the address allowed to withdraw for the staker
    if let Staker::Local { address } = &staker {
        ensure_sender(&info, address)?;
    };

    let Some(batch) = deps.storage.maybe_read::<Batches>(&batch_id)? else {
        return Err(ContractError::BatchNotFound { batch_id });
    };

    let BatchState::Received {
        received_native_unstaked,
    } = batch.state
    else {
        return Err(ContractError::BatchNotYetReceived { batch_id });
    };

    let unstake_request_key = UnstakeRequestKey {
        batch_id,
        staker_hash: staker.hash(),
    };
    let liquid_unstake_request = deps
        .storage
        .maybe_read::<UnstakeRequests>(&unstake_request_key)?
        .ok_or_else(|| ContractError::NoRequestInBatch {
            batch_id,
            staker: staker.clone(),
        })?;

    let amount = Uint128::new(received_native_unstaked)
        .multiply_ratio(liquid_unstake_request.amount, batch.total_lst_to_burn)
        .u128();

    // delete unstake request from both maps
    deps.storage.delete::<UnstakeRequests>(&unstake_request_key);
    deps.storage
        .delete::<UnstakeRequestsByStakerHash>(&unstake_request_key);

    Ok(Response::new()
        .add_event(
            Event::new("withdraw")
                .add_attribute("batch", batch_id.to_string())
                .add_attribute("amount", amount.to_string()),
        )
        // send the native token (U) back to the staker
        .add_message(BankMsg::Send {
            to_address: withdraw_to_address.to_string(),
            amount: vec![Coin {
                denom: config.native_token_denom.clone(),
                amount: amount.into(),
            }],
        }))
}

// Transfer ownership to another account; callable by the owner
// This will require the new owner to accept to take effect.
// No need to handle case of overwriting the pending owner
// Ownership can only be claimed after 7 days to mitigate fat finger errors
pub fn transfer_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_owner: String,
) -> ContractResult<Response> {
    ensure_admin(deps.as_ref(), &info)?;

    deps.storage
        .upsert_item::<PendingOwnerStore, ContractError>(|pending_owner| {
            let mut pending_owner = pending_owner.unwrap();
            pending_owner.address = new_owner.clone();
            pending_owner.owner_transfer_min_time_seconds =
                env.block.time.seconds() + OWNERSHIP_CLAIM_DELAY_PERIOD;

            Ok(pending_owner)
        })?;

    Ok(Response::new().add_event(
        Event::new("transfer_ownership")
            .add_attribute("new_owner", new_owner)
            .add_attribute("previous_owner", info.sender),
    ))
}

// Revoke transfer ownership, callable by the owner
pub fn revoke_ownership_transfer(deps: DepsMut, info: MessageInfo) -> ContractResult<Response> {
    ensure_admin(deps.as_ref(), &info)?;

    deps.storage.delete_item::<PendingOwnerStore>();

    Ok(Response::new().add_event(Event::new("revoke_ownership_transfer")))
}

pub fn accept_ownership(deps: DepsMut, env: Env, info: MessageInfo) -> ContractResult<Response> {
    let PendingOwner {
        address: pending_owner,
        owner_transfer_min_time_seconds,
    } = deps
        .storage
        .maybe_read_item::<PendingOwnerStore>()?
        .ok_or(ContractError::NoPendingOwner)?;

    if owner_transfer_min_time_seconds > env.block.time.seconds() {
        return Err(ContractError::OwnershipTransferNotReady {
            time_to_claim_seconds: owner_transfer_min_time_seconds,
        });
    }

    if pending_owner == info.sender.as_str() {
        deps.storage.delete_item::<PendingOwnerStore>();
        deps.storage.write_item::<Admin>(&info.sender);
        Ok(Response::new().add_event(
            Event::new("accept_ownership").add_attributes([attr("new_owner", info.sender)]),
        ))
    } else {
        Err(ContractError::CallerIsNotPendingOwner)
    }
}

// TODO: Implement once basic functionality is completed
// // Update the config; callable by the owner
// #[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    protocol_fee_config: Option<ProtocolFeeConfig>,
    monitors: Option<Vec<Addr>>,
    batch_period_seconds: Option<u64>,
) -> ContractResult<Response> {
    ensure_admin(deps.as_ref(), &info)?;

    let mut event = Event::new("update_config");

    if let Some(protocol_fee_config) = protocol_fee_config {
        deps.storage
            .write_item::<ProtocolFeeConfigStore>(&protocol_fee_config);

        event = event
            .add_attribute(
                "protocol_fee_rate",
                protocol_fee_config.fee_rate.to_string(),
            )
            .add_attribute(
                "protocol_fee_recpient",
                protocol_fee_config.fee_recipient.to_string(),
            );
    }

    if let Some(monitors) = monitors {
        // collect to Vec<String> as that's the type used in the store (and .join() can be called on it as well)
        let monitors = monitors.into_iter().map(Into::into).collect();
        deps.storage.write_item::<Monitors>(&monitors);
        event = event.add_attribute("monitors", format!("[{}]", monitors.join(",")));
    }

    if let Some(batch_period_seconds) = batch_period_seconds {
        let unbonding_period =
            query_and_validate_unbonding_period(deps.as_ref(), batch_period_seconds)?;

        deps.storage
            .upsert_item::<ConfigStore, ContractError>(|config| {
                Ok(Config {
                    batch_period_seconds,
                    ..config.unwrap()
                })
            })?;

        event = event
            .add_attribute("batch_period_seconds", batch_period_seconds.to_string())
            .add_attribute("current_unbonding_period", unbonding_period.to_string());
    }

    Ok(Response::new().add_event(event))
}

/// Receive rewards (denominated in the native token) to this contract.
///
/// The native token is the reward token for U<->eU. Anyone is able to call this entrypoint to increase the backing of the LST (eU).
///
/// - Send native token to the contract
/// - Accrue (rewards, fees) based on the amount of rewards sent
pub fn receive_rewards(deps: DepsMut, info: MessageInfo) -> ContractResult<Response> {
    let config = deps.storage.read_item::<ConfigStore>()?;

    ensure_not_stopped(deps.as_ref())?;

    let received_rewards = must_pay(&info, &config.native_token_denom)?.u128();

    let protocol_fee_config = deps.storage.read_item::<ProtocolFeeConfigStore>()?;

    let protocol_fee = Uint128::new(protocol_fee_config.fee_rate)
        .multiply_ratio(received_rewards, FEE_RATE_DENOMINATOR)
        .u128();

    if protocol_fee == 0 {
        return Err(ContractError::ComputedFeesAreZero { received_rewards });
    }

    let amount_after_protocol_fee =
        received_rewards.checked_sub(protocol_fee).ok_or_else(|| {
            ContractError::RewardsReceivedLessThanProtocolFee {
                received_rewards,
                protocol_fee,
            }
        })?;

    deps.storage
        .upsert_item::<AccountingStateStore, ContractError>(|accounting_state| {
            let mut accounting_state = accounting_state.unwrap();
            if accounting_state.total_issued_lst == 0 {
                return Err(ContractError::NoLiquidStake);
            }

            // update the accounting of tokens
            accounting_state.total_bonded_native_tokens += amount_after_protocol_fee;
            accounting_state.total_reward_amount += received_rewards;

            Ok(accounting_state)
        })?;

    Ok(Response::new()
        .add_event(
            Event::new("receive_rewards")
                .add_attribute("amount", received_rewards.to_string())
                .add_attribute(
                    "amount_after_protocol_fee",
                    amount_after_protocol_fee.to_string(),
                )
                .add_attribute("protocol_fee", protocol_fee.to_string()),
        )
        // send amount after fees to the staker
        .add_message(BankMsg::Send {
            to_address: deps.storage.read_item::<StakerAddress>()?.to_string(),
            amount: vec![Coin::new(
                amount_after_protocol_fee,
                &config.native_token_denom,
            )],
        })
        // send fees to the fee recipient
        .add_message(BankMsg::Send {
            to_address: protocol_fee_config.fee_recipient.to_string(),
            amount: vec![cosmwasm_std::Coin::new(
                protocol_fee,
                config.native_token_denom,
            )],
        }))
}

/// Marks a batch as received
/// Public function? Permissionless?
pub fn receive_unstaked_tokens(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    batch_id: BatchId,
) -> ContractResult<Response> {
    let config = deps.storage.read_item::<ConfigStore>()?;

    ensure_not_stopped(deps.as_ref())?;

    let amount = must_pay(&info, &config.native_token_denom)?.u128();

    deps.storage
        .upsert::<Batches, ContractError>(&batch_id, |batch| {
            let mut batch = batch.ok_or(ContractError::BatchNotFound { batch_id })?;

            let expected_native_unstaked = match batch.state {
                BatchState::Pending { .. } => {
                    return Err(ContractError::BatchStillPending { batch_id })
                }
                BatchState::Submitted {
                    receive_time,
                    expected_native_unstaked,
                } => {
                    ensure!(
                        receive_time <= env.block.time.seconds(),
                        ContractError::BatchNotReady {
                            now: env.block.time.seconds(),
                            ready_at: receive_time,
                        }
                    );
                    expected_native_unstaked
                }
                BatchState::Received { .. } => {
                    return Err(ContractError::BatchAlreadyReceived { batch_id })
                }
            };

            ensure!(
                expected_native_unstaked == amount,
                ContractError::ReceivedWrongBatchAmount {
                    batch_id,
                    expected: expected_native_unstaked,
                    received: amount,
                }
            );

            batch.state = BatchState::Received {
                received_native_unstaked: amount,
            };

            Ok(batch)
        })?;

    Ok(Response::new().add_event(
        Event::new("receive_unstaked_tokens")
            .add_attribute("batch", batch_id.to_string())
            .add_attribute("amount", amount.to_string()),
    ))
}

pub fn circuit_breaker(deps: DepsMut, info: MessageInfo) -> ContractResult<Response> {
    // must either be admin or a monitor to halt the contract
    if deps.storage.read_item::<Admin>()? == info.sender
        || deps
            .storage
            .read_item::<Monitors>()?
            .contains(&info.sender.to_string())
    {
        deps.storage.write_item::<Stopped>(&true);

        Ok(Response::new().add_event(Event::new("circuit_breaker")))
    } else {
        Err(ContractError::Unauthorized {
            sender: info.sender,
        })
    }
}

pub fn resume_contract(
    deps: DepsMut,
    info: MessageInfo,
    new_accounting_state: AccountingState,
) -> ContractResult<Response> {
    ensure_admin(deps.as_ref(), &info)?;
    ensure_stopped(deps.as_ref())?;

    deps.storage.write_item::<Stopped>(&false);

    deps.storage
        .write_item::<AccountingStateStore>(&new_accounting_state);

    Ok(Response::new().add_event(
        Event::new("resume_contract")
            .add_attribute(
                "total_bonded_native_tokens",
                new_accounting_state.total_bonded_native_tokens.to_string(),
            )
            .add_attribute(
                "total_issued_lst",
                new_accounting_state.total_issued_lst.to_string(),
            )
            .add_attribute(
                "total_reward_amount",
                new_accounting_state.total_reward_amount.to_string(),
            ),
    ))
}

pub fn slash_batches(
    deps: DepsMut,
    info: MessageInfo,
    expected_amounts: Vec<BatchExpectedAmount>,
) -> ContractResult<Response> {
    ensure_admin(deps.as_ref(), &info)?;

    // ensure the contract is stopped before slashing the batches
    ensure_stopped(deps.as_ref())?;

    for BatchExpectedAmount {
        batch_id,
        expected_native_amount,
    } in &expected_amounts
    {
        deps.storage
            .upsert::<Batches, ContractError>(batch_id, |batch| {
                let mut batch = batch.ok_or_else(|| ContractError::BatchNotFound {
                    batch_id: *batch_id,
                })?;

                let BatchState::Submitted {
                    ref mut expected_native_unstaked,
                    ..
                } = batch.state
                else {
                    return Err(ContractError::BatchNotYetSubmitted {
                        batch_id: *batch_id,
                    });
                };

                *expected_native_unstaked = *expected_native_amount;

                Ok(batch)
            })?;
    }

    Ok(Response::new().add_events(expected_amounts.into_iter().map(
        |BatchExpectedAmount {
             batch_id,
             expected_native_amount: amount,
         }| {
            Event::new("slash_batch")
                .add_attribute("batch_id", batch_id.to_string())
                .add_attribute("amount", amount.to_string())
        },
    )))
}
