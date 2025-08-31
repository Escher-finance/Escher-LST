use cosmwasm_std::{
    attr, ensure, wasm_execute, Addr, BankMsg, Coin, Deps, DepsMut, Env, Event, MessageInfo,
    Response, Timestamp,
};
use cw20::Cw20ExecuteMsg;
use cw_utils::must_pay;
use depolama::StorageExt;
use ibc_union_spec::ChannelId;
use unionlabs_primitives::{Bytes, U256};

use crate::{
    error::{ContractError, ContractResult},
    helpers::{compute_mint_amount, compute_unbond_amount, query_and_validate_unbonding_period},
    state::{
        AccountingStateStore, Admin, Batches, ConfigStore, LstAddress, Monitors, OnZkgmCallProxy,
        PendingBatchId, StakerAddress, Stopped, UnstakeRequests,
    },
    types::{
        AccountingState, Batch, BatchExpectedAmount, BatchId, BatchState, Config, Staker,
        UnstakeRequest, UnstakeRequestKey,
    },
};

const FEE_RATE_DENOMINATOR: u64 = 100_000;

pub fn ensure_not_stopped(deps: Deps) -> Result<(), ContractError> {
    if deps.storage.read_item::<Stopped>()? {
        return Err(ContractError::Stopped);
    }
    Ok(())
}

// TODO: Build out an allowances system?
pub fn ensure_trusted_address(
    deps: Deps,
    info: &MessageInfo,
    local_staker_address: &str,
) -> Result<(), ContractError> {
    if info.sender.as_bytes() != local_staker_address {
        let on_zkgm_call_proxy_address = deps.storage.read_item::<OnZkgmCallProxy>()?;

        // on_zkgm_call_proxy is a trusted address
        if info.sender != on_zkgm_call_proxy_address {
            return Err(ContractError::Unauthorized {
                sender: info.sender.clone(),
            });
        }
    }

    Ok(())
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
        return Err(ContractError::MintError);
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
        ensure_trusted_address(deps.as_ref(), &info, &address)?;
    };

    let pending_batch_id = deps.storage.read_item::<PendingBatchId>()?;

    let staker_hash = staker.hash();

    // 1.
    let mut is_new_request = false;
    deps.storage.upsert::<UnstakeRequests, ContractError>(
        &UnstakeRequestKey {
            batch_id: pending_batch_id,
            staker_hash,
        },
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
    );

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
                .add_attribute("action", "unbond")
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

    let mut batch = deps.storage.read::<Batches>(&pending_batch_id)?;

    let BatchState::Pending { submit_time } = batch.state else {
        return Err(ContractError::BatchAlreadySubmitted);
    };

    ensure!(
        env.block.time.seconds() >= submit_time,
        ContractError::BatchNotReady {
            actual: env.block.time.seconds(),
            expected: submit_time,
        }
    );

    ensure!(batch.unstake_requests_count > 0, ContractError::BatchEmpty);

    let mut accounting_state = deps.storage.read_item::<AccountingStateStore>()?;

    ensure!(
        accounting_state.total_issued_lst >= batch.total_lst_to_burn,
        ContractError::InvalidUnstakeAmount {
            total_liquid_stake_token: accounting_state.total_issued_lst,
            amount_to_unstake: batch.total_lst_to_burn
        }
    );

    let unbond_amount = compute_unbond_amount(
        accounting_state.total_bonded_native_tokens,
        accounting_state.total_issued_lst,
        batch.total_lst_to_burn,
    );

    // reduce underlying native token balance by unbonded amount
    accounting_state.total_bonded_native_tokens = accounting_state
        .total_bonded_native_tokens
        .checked_sub(unbond_amount)
        .unwrap_or_default();

    // reduce underlying LST balance by batch total
    accounting_state.total_issued_lst = accounting_state
        .total_issued_lst
        .checked_sub(batch.total_lst_to_burn)
        .unwrap_or_default();

    let unbonding_period =
        query_and_validate_unbonding_period(deps.as_ref(), config.batch_period_seconds)?;

    batch.state = BatchState::Submitted {
        receive_time: env.block.time.seconds() + unbonding_period,
        expected_native_unstaked: unbond_amount,
    };

    deps.storage.write::<Batches>(&pending_batch_id, &batch);
    deps.storage
        .write_item::<AccountingStateStore>(&accounting_state);

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
                // REVIEW: Remove this "action" attribute?
                .add_attribute("action", "submit_batch")
                .add_attribute("batch_id", pending_batch_id.to_string())
                .add_attribute("batch_total", batch.total_lst_to_burn.to_string())
                .add_attribute("expected_unstaked", unbond_amount.to_string())
                .add_attribute("unbonding_period", unbonding_period.to_string()),
        ))
}

pub fn withdraw(
    mut deps: DepsMut,
    info: MessageInfo,
    batch_id: BatchId,
    staker: Staker,
) -> ContractResult<Response> {
    let config = deps.storage.read_item::<ConfigStore>()?;

    ensure_not_stopped(deps.as_ref())?;

    ensure_trusted_address(&config, &info, &staker)?;

    let Some(batch) = deps.storage.maybe_read::<Batches>(&batch_id)? else {
        return Err(ContractError::BatchNotFound { batch_id });
    };

    let BatchState::Received {
        received_native_unstaked,
    } = batch.state
    else {
        return Err(ContractError::BatchNotYetReceived);
    };

    let liquid_unstake_request = deps
        .storage
        .maybe_read(&UnstakeRequestKey {
            batch_id,
            staker_hash: staker.hash(),
        })?
        .ok_or_else(|| ContractError::NoRequestInBatch {
            staker: staker.clone(),
        })?;

    let amount = received_native_unstaked
        .multiply_ratio(liquid_unstake_request.amount, batch.total_lst_to_burn);

    remove_unstake_request(&mut deps, staker.clone(), batch_id)?;

    Ok(Response::new()
        // send the native token (U) back to the staker
        .add_message(BankMsg::Send {
            to_address: todo!("allow withdrawal to other channels"),
            amount: vec![Coin {
                denom: config.native_token_denom.clone(),
                amount,
            }],
        })
        .add_attribute("action", "execute_withdraw")
        .add_attribute("batch", batch_id.to_string())
        .add_attribute("amount", amount.to_string()))
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
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    STATE.update::<_, ContractError>(deps.storage, |mut state| {
        state.pending_owner = Some(deps.api.addr_validate(&new_owner)?);
        state.owner_transfer_min_time = Some(Timestamp::from_seconds(
            env.block.time.seconds() + 60 * 60 * 24 * 7,
        )); // 7 days

        Ok(state)
    })?;

    Ok(Response::new()
        .add_attribute("action", "transfer_ownership")
        .add_attribute("new_owner", new_owner)
        .add_attribute("previous_owner", info.sender))
}

// Revoke transfer ownership, callable by the owner
pub fn revoke_ownership_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> ContractResult<Response> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    STATE.update::<_, ContractError>(deps.storage, |mut state| {
        state.pending_owner = None;
        state.owner_transfer_min_time = None;

        Ok(state)
    })?;

    Ok(Response::new().add_attribute("action", "revoke_ownership_transfer"))
}

pub fn accept_ownership(deps: DepsMut, env: Env, info: MessageInfo) -> ContractResult<Response> {
    let mut accounting = deps.storage.read_item::<AccountingStateStore>()?;

    if let Some(owner_transfer_min_time) = accounting.owner_transfer_min_time {
        if owner_transfer_min_time > env.block.time {
            return Err(ContractError::OwnershipTransferNotReady {
                time_to_claim: Timestamp::from_seconds(
                    accounting.owner_transfer_min_time.unwrap().seconds(),
                ),
            });
        }
    }

    match accounting.pending_owner {
        Some(pending_owner) => {
            if pending_owner == info.sender {
                accounting.pending_owner = None;
                STATE.save(deps.storage, &accounting)?;

                ADMIN.set(deps, Some(pending_owner))?;
                Ok(Response::new()
                    .add_attribute("action", "accept_ownership")
                    .add_attribute("new_owner", info.sender))
            } else {
                Err(ContractError::CallerIsNotPendingOwner)
            }
        }
        _ => Err(ContractError::NoPendingOwner),
    }
}

// TODO: Implement once basic functionality is completed
// // Update the config; callable by the owner
// #[allow(clippy::too_many_arguments)]
// pub fn update_config(
//     deps: DepsMut,
//     _env: Env,
//     info: MessageInfo,
//     protocol_fee_config: Option<UnsafeProtocolFeeConfig>,
//     monitors: Option<Vec<String>>,
//     batch_period: Option<u64>,
// ) -> ContractResult<Response> {
//     ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

//     let mut config = CONFIG.load(deps.storage)?;

//     if let Some(native_chain_config) = &native_chain_config {
//         config = native_chain_config.validate()?;
//     }

//     if let Some(protocol_chain_config) = protocol_chain_config {
//         config = protocol_chain_config.validate(&config.token_denom)?;
//     }

//     // The native chain config contains the native token denom,
//     // which influences protocol_chain_config.ibc_token_denom.
//     // Ensure that if the native token denom has changed,
//     // the configured IBC denom remains valid after updating protocol_chain_config.
//     if native_chain_config.is_some() {
//         validate_ibc_denom(
//             &config.native_token_denom,
//             &config.ibc_channel_id,
//             &config.token_denom,
//         )?;
//     }

//     if let Some(protocol_fee_config) = protocol_fee_config {
//         config.protocol_fee_config = protocol_fee_config.validate(&config)?
//     }

//     if let Some(monitors) = monitors {
//         config.monitors = validate_addresses(&monitors, &config.account_address_prefix)?;
//     }

//     if let Some(batch_period) = batch_period {
//         // Ensure the batch period is lower then unbonding period.
//         if batch_period > config.unbonding_period {
//             return Err(ContractError::ValueTooBig {
//                 field_name: "batch_period".to_string(),
//                 value: Uint128::from(config.unbonding_period),
//                 max: Uint128::from(batch_period),
//             });
//         }
//         config.batch_period = batch_period;
//     }

//     CONFIG.save(deps.storage, &config)?;

//     Ok(Response::new().add_attribute("action", "update_config"))
// }

/// Receive rewards (denominated in the native token) to this contract.
///
/// The native token is the reward token for U<->eU. Anyone is able to call this entrypoint to increase the backing of the LST (eU).
///
/// - Send native token to the contract
/// - Accrue (rewards, fees) based on the amount of rewards sent
pub fn receive_rewards(deps: DepsMut, info: MessageInfo) -> ContractResult<Response> {
    let config = deps.storage.read_item::<ConfigStore>()?;

    ensure_not_stopped(deps.as_ref())?;

    let amount = must_pay(&info, &config.native_token_denom)?;

    let fee = config
        .protocol_fee_config
        .fee_rate
        .multiply_ratio(amount, FEE_RATE_DENOMINATOR);
    if fee == 0 {
        return Err(ContractError::ComputedFeesAreZero {
            received_rewards: amount,
        });
    }
    let amount_after_fees =
        amount
            .checked_sub(fee)
            .map_err(|_| ContractError::ReceiveRewardsTooSmall {
                amount,
                minimum: fee,
            })?;

    STATE.update::<_, ContractError>(deps.storage, |mut state| {
        if state.total_issued_lst == 0 {
            return Err(ContractError::NoLiquidStake);
        }

        // update the accounting of tokens
        state.total_bonded_native_tokens += amount_after_fees;
        state.total_reward_amount += amount;

        Ok(state)
    })?;

    Ok(Response::new()
        .add_attribute("action", "receive_rewards")
        .add_attribute("action", "transfer_stake")
        .add_attribute("amount", amount)
        .add_attribute("amount_after_fees", amount_after_fees)
        // send amount after fees to the staker
        .add_message(BankMsg::Send {
            to_address: config.staker_address.to_string(),
            amount: vec![Coin::new(amount_after_fees, &config.native_token_denom)],
        })
        // send fees to the fee recipient
        .add_message(BankMsg::Send {
            to_address: config.protocol_fee_config.fee_recipient.to_string(),
            amount: vec![cosmwasm_std::Coin::new(fee, config.native_token_denom)],
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
                BatchState::Pending { .. } => return Err(ContractError::BatchStillPending),
                BatchState::Submitted {
                    receive_time,
                    expected_native_unstaked,
                } => {
                    ensure!(
                        receive_time <= env.block.time.seconds(),
                        ContractError::BatchNotReady {
                            actual: env.block.time.seconds(),
                            expected: receive_time,
                        }
                    );
                    expected_native_unstaked
                }
                BatchState::Received { .. } => return Err(ContractError::BatchAlreadyReceived),
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

    Ok(Response::new()
        .add_attribute("action", "receive_unstaked_tokens")
        .add_attribute("batch", batch_id.to_string())
        .add_attribute("amount", amount.to_string()))
}

pub fn circuit_breaker(deps: DepsMut, _env: Env, info: MessageInfo) -> ContractResult<Response> {
    let sender = info.sender.to_string();

    // must either be admin or a monitor to halt the contract
    if deps.storage.read_item::<Admin>()? == info.sender
        || deps.storage.maybe_read::<Monitors>(&info.sender)?.is_some()
    {
        deps.storage.write_item::<Stopped>(&true);

        Ok(Response::new().add_attribute("action", "circuit_breaker"))
    } else {
        Err(ContractError::Unauthorized { sender })
    }
}

pub fn resume_contract(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    total_bonded_native_tokens: u128,
    total_liquid_stake_token: u128,
    total_reward_amount: u128,
) -> ContractResult<Response> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    CONFIG.update::<_, ContractError>(deps.storage, |mut config| {
        if !config.stopped {
            return Err(ContractError::NotStopped);
        }

        config.stopped = false;

        Ok(config)
    })?;

    STATE.update::<_, ContractError>(deps.storage, |mut state| {
        state.total_bonded_native_tokens = total_bonded_native_tokens;
        state.total_issued_lst = total_liquid_stake_token;
        state.total_reward_amount = total_reward_amount;

        Ok(state)
    })?;

    Ok(Response::new()
        .add_attribute("action", "resume_contract")
        .add_attribute("total_bonded_native_tokens", total_bonded_native_tokens)
        .add_attribute("total_liquid_stake_token", total_liquid_stake_token)
        .add_attribute("total_reward_amount", total_reward_amount))
}

// pub fn slash_batches(
//     deps: DepsMut,
//     info: MessageInfo,
//     expected_amounts: Vec<BatchExpectedAmount>,
// ) -> ContractResult<Response> {
//     ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

//     // Ensure the contract is stopped before slashing the batches
//     if !CONFIG.load(deps.storage)?.stopped {
//         return Err(ContractError::NotStopped);
//     }

//     for BatchExpectedAmount { batch_id, amount } in expected_amounts.iter() {
//         let mut batch = BATCHES.load(deps.storage, *batch_id)?;
//         let BatchState::Submitted {
//             ref mut expected_native_unstaked,
//             ..
//         } = batch.state
//         else {
//             return Err(ContractError::BatchNotYetSubmitted {
//                 batch_id: *batch_id,
//             });
//         };

//         *expected_native_unstaked = *amount;

//         deps.storage.write::<Batches>(*batch_id, &batch)?;
//     }

//     Ok(Response::new()
//         .add_attribute("action", "slash_batches")
//         .add_attribute("updated_batches", serde_json::to_string(&expected_amounts)?))
// }
