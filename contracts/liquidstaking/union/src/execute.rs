use alloy::sol_types::SolValue;
use cosmwasm_std::{
    ensure, wasm_execute, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, StdError, Timestamp,
    Uint128,
};
use cw20::Cw20ExecuteMsg;
use cw_utils::must_pay;
use ibc_union_spec::ChannelId;
use milky_way::staking::{Batch, BatchState};
use ucs03_zkgm::com::{Instruction, TokenOrderV2, INSTR_VERSION_2, OP_TOKEN_ORDER};
use unionlabs_primitives::Bytes;

use crate::{
    error::{ContractError, ContractResult},
    helpers::{compute_mint_amount, compute_unbond_amount, query_and_validate_unbonding_period},
    state::{
        new_unstake_request, remove_unstake_request, unstake_requests, Config, UnstakeRequest,
        ADMIN, BATCHES, CONFIG, FUNGIBLE_RECIPIENT_CHANNEL, PENDING_BATCH_ID, STATE,
    },
    types::BatchExpectedAmount,
};

const FEE_RATE_DENOMINATOR: u64 = 100_000;

pub fn check_stopped(config: &Config) -> Result<(), ContractError> {
    if config.stopped {
        return Err(ContractError::Stopped);
    }
    Ok(())
}

// TODO: Build out an allowances system?
pub fn ensure_trusted_address(
    config: &Config,
    info: &MessageInfo,
    staker: &str,
) -> Result<(), ContractError> {
    if info.sender.as_str() != staker {
        // funded_dispatch_address is a trusted address
        if info.sender != config.funded_dispatch_address {
            panic!("nice try fucker")
        }
    }

    Ok(())
}

// PENDING
// Payment validation handled by caller (not sure what this means)
// Denom validation handled by caller (done in contract.rs)
/// # Changes from original implementation
///
/// - `mint_to` is no longer optional, and must be provided on every call.
/// - `transfer_to_native_chain` and the `mint_to_is_native` `mint_to_is_protocol` logic has been removed. this contract will be running on chain where the staking will be taking place, so there is no need to differentiate between native and protocol chain.
/// - added `recipient_channel_id`, which if set will automatically transfer the minted lst tokens over the specified channel to the `mint_to` parameter.
/// - renamed `expected_mint_amount` to `min_mint_amount` for clarity as this matches the semantics of the value (mint_amount >= min_mint_amount)
/// - oracle functionality has been removed
/// - tokenfactory functionality has been converted into the equivalent cw20 messages
/// - ibc sub messages have been converted to the equivalent cw20 messages (since the transfers will happen on the same chain, not cross-chain back to the native chain)
pub fn execute_bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    // NOTE: In the original milkyway implementation, this was optional and defaulted to info.sender if not set. Due to how we will be using this contract (i.e. with the funded-dispatch contract in the zkgm batch), it is much clearer to enforce this to be set. This makes it slightly more verbose for a user who is liquid staking directly on union, but we deem this to be a bit of a degenerate case as this will only be done in the ui from ethereum (via zkgm).
    mint_to_address: Bytes,
    recipient_channel_id: Option<ChannelId>,
    min_mint_amount: Option<Uint128>,
) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    check_stopped(&config)?;

    let amount = must_pay(&info, &config.native_token_denom)?;

    let mut state = STATE.load(deps.storage)?;
    ensure!(
        amount >= config.minimum_liquid_stake_amount,
        ContractError::MinimumLiquidStakeAmount {
            minimum_stake_amount: config.minimum_liquid_stake_amount,
            sent_amount: amount,
        }
    );

    // Compute mint amount
    let mint_amount = compute_mint_amount(
        state.total_native_token,
        state.total_liquid_stake_token,
        amount,
    );

    // If mint amount is zero it is likely there was a an issue with rounding, return error and do not mint
    if mint_amount.is_zero() {
        return Err(ContractError::MintError);
    }

    if let Some(min_mint_amount) = min_mint_amount {
        ensure!(
            mint_amount >= min_mint_amount,
            ContractError::MintAmountMismatch {
                expected: min_mint_amount,
                actual: mint_amount
            }
        );
    }

    // transfer native token to multisig address
    // NOTE: In the original milkyway satking contracts, this was an ibc transfer message back to the multisig on the source chain since the liquid staking is *initiated* on the protocol chain (i.e. where this contract is deployed), but the *staking* happens on the native chain
    // TODO: This should be a wasm_execute call with a specific ExecuteMsg for the staker contract
    // TODO: Define the ExecuteMsg for the staker contract
    let transfer_funds_to_cw_account_msig_message = BankMsg::Send {
        to_address: config.staker_address.to_string(),
        amount: info.funds,
    };
    state.total_native_token += amount;
    state.total_liquid_stake_token += mint_amount;
    STATE.save(deps.storage, &state)?;

    let response = Response::new()
        .add_message(transfer_funds_to_cw_account_msig_message)
        .add_attribute("mint_to_address", mint_to_address.to_string())
        .add_attribute("action", "bond")
        // NOTE: In practice, this will always be the funded-dispatch contract. This may need to be changed to emit the original sender from the source chain (if it exists).
        .add_attribute("sender", info.sender.to_string())
        .add_attribute("in_amount", amount)
        .add_attribute("mint_amount", mint_amount);

    let response = if let Some(recipient_channel_id) = recipient_channel_id {
        let recipient_channel_config =
            FUNGIBLE_RECIPIENT_CHANNEL.load(deps.storage, recipient_channel_id.raw())?;

        // transfer the eU to the desired recipient channel
        //
        // NOTE: We can't just mint into zkgm directly, since zkgm uses `transfer_from` to spend on behalf of the caller, *not* `transfer` (which would spend zkgm's own balance).
        response.add_messages([
            // mint LST to self
            wasm_execute(
                &config.liquid_stake_token_address,
                &Cw20ExecuteMsg::Mint {
                    amount: mint_amount,
                    recipient: env.contract.address.to_string(),
                },
                vec![],
            )?,
            // increase allowance of eU for zkgm from self
            wasm_execute(
                // eU address
                &config.liquid_stake_token_address,
                &Cw20ExecuteMsg::IncreaseAllowance {
                    spender: config.ucs03_zkgm_address.to_string(),
                    amount: mint_amount,
                    expires: None,
                },
                vec![],
            )?,
            wasm_execute(
                &config.ucs03_zkgm_address,
                &ucs03_zkgm::msg::ExecuteMsg::Send {
                    channel_id: recipient_channel_id,
                    timeout_timestamp: ibc_union_spec::Timestamp::from_nanos(
                        // REVIEW: Make the timeout a config param?
                        // REVIEW: What happens if this packet times out? The tokens will be sent back to this contract?
                        env.block.time.plus_days(3).nanos(),
                    ),
                    timeout_height: 0_u64.into(),
                    // salt is empty as the timestamp will make the packet unique
                    salt: Default::default(),
                    instruction: Instruction {
                        version: INSTR_VERSION_2,
                        opcode: OP_TOKEN_ORDER,
                        operand: TokenOrderV2 {
                            // in the event of a timeout or failure on the destination chain, the funds will be refunded to the staker account.
                            sender: config.staker_address.to_string().into_bytes().into(),
                            receiver: mint_to_address.into(),
                            base_token: config
                                .liquid_stake_token_address
                                .clone()
                                .into_bytes()
                                .into(),
                            base_amount: alloy::primitives::U256::from(mint_amount.u128()),
                            quote_token: recipient_channel_config.quote_token.into(),
                            quote_amount: alloy::primitives::U256::from(mint_amount.u128()),
                            kind: recipient_channel_config.kind,
                            metadata: recipient_channel_config.metadata.into(),
                        }
                        .abi_encode_params()
                        .into(),
                    }
                    .abi_encode_params()
                    .into(),
                },
                vec![],
            )?,
        ])
    } else {
        // if recipient_channel_id is None, then mint_to is expected to be a valid address for this chain
        let recipient = deps.api.addr_validate(
            str::from_utf8(&mint_to_address)
                .map_err(|_| StdError::invalid_utf8("mint_to address is not valid utf8"))?,
        )?;
        // send the minted lst tokens to the user on this network
        response.add_message(wasm_execute(
            // eU address
            config.liquid_stake_token_address,
            &Cw20ExecuteMsg::Mint {
                amount: mint_amount,
                recipient: recipient.to_string(),
            },
            vec![],
        )?)
    };

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
pub fn execute_unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    staker: String,
) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    check_stopped(&config)?;

    ensure_trusted_address(&config, &info, &staker)?;

    let pending_batch_id = PENDING_BATCH_ID.load(deps.storage)?;

    // 1.
    let mut is_new_request = false;
    unstake_requests().update(
        deps.storage,
        (pending_batch_id, staker.clone()),
        |or| -> Result<UnstakeRequest, ContractError> {
            Ok(match or {
                Some(r) => UnstakeRequest {
                    batch_id: r.batch_id,
                    user: r.user.clone(),
                    amount: r.amount + amount,
                },
                None => {
                    // this is a bit of a hack since .update() doesn't allow for returning anything else other than the T of the IndexMap. ideally it would return (R, T), write T to storage, and then return (R, T)
                    is_new_request = true;
                    UnstakeRequest {
                        batch_id: pending_batch_id,
                        user: staker.clone(),
                        amount,
                    }
                }
            })
        },
    )?;

    // 2.
    BATCHES.update(
        deps.storage,
        pending_batch_id,
        |batch| -> Result<Batch, ContractError> {
            let mut batch = batch.unwrap();
            batch.batch_total_liquid_stake += amount;
            if is_new_request {
                batch.unstake_requests_count += 1;
            }
            Ok(batch)
        },
    )?;

    // 3.
    let lst_transfer_from_msg = wasm_execute(
        &config.liquid_stake_token_address,
        &Cw20ExecuteMsg::TransferFrom {
            owner: staker.clone(),
            recipient: env.contract.address.to_string(),
            amount,
        },
        vec![],
    )?;

    Ok(Response::new()
        .add_message(lst_transfer_from_msg)
        .add_attribute("action", "unbond")
        .add_attribute("sender", staker)
        .add_attribute("batch", pending_batch_id.to_string())
        .add_attribute("amount", amount)
        .add_attribute("is_new_request", is_new_request.to_string()))
}

/// Submit batch and transition pending batch to submitted.
///
/// # Changes from original implementation
///
/// - oracle functionality was removed
///
/// TODO: Withdraw unstaked tokens in this function
pub fn execute_submit_batch(deps: DepsMut, env: Env) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    check_stopped(&config)?;

    let pending_batch_id = PENDING_BATCH_ID.load(deps.storage)?;
    let mut batch = BATCHES.load(deps.storage, pending_batch_id)?;

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

    ensure!(
        state.total_liquid_stake_token >= batch.batch_total_liquid_stake,
        ContractError::InvalidUnstakeAmount {
            total_liquid_stake_token: state.total_liquid_stake_token,
            amount_to_unstake: batch.batch_total_liquid_stake
        }
    );

    // create new pending batch
    let new_pending_batch = Batch::new_pending(
        Uint128::zero(),
        env.block.time.seconds() + config.batch_period,
    );

    // Save new pending batch
    BATCHES.save(deps.storage, &pending_batch_id + 1, &new_pending_batch)?;
    PENDING_BATCH_ID.save(deps.storage, &(pending_batch_id + 1))?;

    let unbond_amount = compute_unbond_amount(
        state.total_native_token,
        state.total_liquid_stake_token,
        batch.batch_total_liquid_stake,
    );

    // reduce underlying native token balance by unbonded amount
    state.total_native_token = state
        .total_native_token
        .checked_sub(unbond_amount)
        .unwrap_or_default();

    // reduce underlying LST balance by batch total
    state.total_liquid_stake_token = state
        .total_liquid_stake_token
        .checked_sub(batch.batch_total_liquid_stake)
        .unwrap_or_default();

    let unbonding_period = query_and_validate_unbonding_period(deps.as_ref(), config.batch_period)?;

    batch.state = BatchState::Submitted {
        receive_time: env.block.time.seconds() + unbonding_period,
        expected_native_unstaked: unbond_amount,
    };

    BATCHES.save(deps.storage, pending_batch_id, &batch)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        // burn all unbonded LST tokens on batch submission
        .add_message(wasm_execute(
            config.liquid_stake_token_address,
            &Cw20ExecuteMsg::Burn {
                amount: batch.batch_total_liquid_stake,
            },
            vec![],
        )?)
        .add_attribute("action", "submit_batch")
        .add_attribute("batch_id", pending_batch_id.to_string())
        .add_attribute("batch_total", batch.batch_total_liquid_stake)
        .add_attribute("expected_native_unstaked", unbond_amount)
        .add_attribute("unbonding_period", unbonding_period.to_string()))
}

pub fn execute_withdraw(
    mut deps: DepsMut,
    info: MessageInfo,
    batch_id: u64,
    staker: String,
) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    check_stopped(&config)?;

    ensure_trusted_address(&config, &info, &staker)?;

    let Some(batch) = BATCHES.may_load(deps.storage, batch_id)? else {
        return Err(ContractError::BatchNotFound { batch_id });
    };

    let BatchState::Received {
        received_native_unstaked,
    } = batch.state
    else {
        return Err(ContractError::BatchNotYetReceived);
    };

    let liquid_unstake_request = unstake_requests()
        .may_load(deps.storage, (batch_id, staker.clone()))?
        .ok_or_else(|| ContractError::NoRequestInBatch {
            staker: staker.clone(),
        })?;

    let amount = received_native_unstaked.multiply_ratio(
        liquid_unstake_request.amount,
        batch.batch_total_liquid_stake,
    );

    remove_unstake_request(&mut deps, staker.clone(), batch_id)?;

    Ok(Response::new()
        // send the native token (U) back to the staker
        .add_message(BankMsg::Send {
            to_address: staker.clone(),
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
pub fn execute_transfer_ownership(
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
pub fn execute_revoke_ownership_transfer(
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

pub fn execute_accept_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> ContractResult<Response> {
    let mut state = STATE.load(deps.storage)?;

    if let Some(owner_transfer_min_time) = state.owner_transfer_min_time {
        if owner_transfer_min_time > env.block.time {
            return Err(ContractError::OwnershipTransferNotReady {
                time_to_claim: Timestamp::from_seconds(
                    state.owner_transfer_min_time.unwrap().seconds(),
                ),
            });
        }
    }

    match state.pending_owner {
        Some(pending_owner) => {
            if pending_owner == info.sender {
                state.pending_owner = None;
                STATE.save(deps.storage, &state)?;

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
    let config = CONFIG.load(deps.storage)?;

    check_stopped(&config)?;

    let amount = must_pay(&info, &config.native_token_denom)?;

    let fee = config
        .protocol_fee_config
        .fee_rate
        .multiply_ratio(amount, FEE_RATE_DENOMINATOR);
    if fee.is_zero() {
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
        if state.total_liquid_stake_token.is_zero() {
            return Err(ContractError::NoLiquidStake);
        }

        // update the accounting of tokens
        state.total_native_token += amount_after_fees;
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
            amount: vec![cosmwasm_std::Coin::new(
                fee.u128(),
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
    batch_id: u64,
) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    check_stopped(&config)?;

    let amount = must_pay(&info, &config.native_token_denom)?;

    BATCHES.update::<_, ContractError>(deps.storage, batch_id, |batch| {
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
        .add_attribute("amount", amount))
}

pub fn circuit_breaker(deps: DepsMut, _env: Env, info: MessageInfo) -> ContractResult<Response> {
    let sender = info.sender.to_string();

    let mut config = CONFIG.load(deps.storage)?;

    if ADMIN.assert_admin(deps.as_ref(), &info.sender).is_err()
        && !config.monitors.iter().any(|v| v.as_str() == sender)
    {
        return Err(ContractError::Unauthorized { sender });
    }

    config.stopped = true;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "circuit_breaker"))
}

/// # Changes from original implementation
///
/// - oracle functionality was removed
pub fn resume_contract(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    total_native_token: Uint128,
    total_liquid_stake_token: Uint128,
    total_reward_amount: Uint128,
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
        state.total_native_token = total_native_token;
        state.total_liquid_stake_token = total_liquid_stake_token;
        state.total_reward_amount = total_reward_amount;

        Ok(state)
    })?;

    Ok(Response::new()
        .add_attribute("action", "resume_contract")
        .add_attribute("total_native_token", total_native_token)
        .add_attribute("total_liquid_stake_token", total_liquid_stake_token)
        .add_attribute("total_reward_amount", total_reward_amount))
}

pub fn slash_batches(
    deps: DepsMut,
    info: MessageInfo,
    expected_amounts: Vec<BatchExpectedAmount>,
) -> ContractResult<Response> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    // Ensure the contract is stopped before slashing the batches
    if !CONFIG.load(deps.storage)?.stopped {
        return Err(ContractError::NotStopped);
    }

    for BatchExpectedAmount { batch_id, amount } in expected_amounts.iter() {
        let mut batch = BATCHES.load(deps.storage, *batch_id)?;
        let BatchState::Submitted {
            ref mut expected_native_unstaked,
            ..
        } = batch.state
        else {
            return Err(ContractError::BatchNotYetSubmitted {
                batch_id: *batch_id,
            });
        };

        *expected_native_unstaked = *amount;

        BATCHES.save(deps.storage, *batch_id, &batch)?;
    }

    Ok(Response::new()
        .add_attribute("action", "slash_batches")
        .add_attribute("updated_batches", serde_json::to_string(&expected_amounts)?))
}
