use alloy::sol_types::SolValue;
use cosmwasm_std::{
    ensure, wasm_execute, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, StdError, Timestamp,
    Uint128,
};
use cw20::Cw20ExecuteMsg;
use cw_utils::PaymentError;
use ibc_union_spec::ChannelId;
use milky_way::staking::{Batch, BatchStatus};
use ucs03_zkgm::com::{
    Instruction, TokenOrderV2, INSTR_VERSION_2, OP_TOKEN_ORDER, TOKEN_ORDER_KIND_ESCROW,
};
use unionlabs_primitives::Bytes;

use crate::{
    error::{ContractError, ContractResult},
    helpers::{compute_mint_amount, compute_unbond_amount},
    state::{
        new_unstake_request, remove_unstake_request, unstake_requests, Config, State,
        UnstakeRequest, ADMIN, BATCHES, CONFIG, PENDING_BATCH_ID, STATE,
    },
    types::BatchExpectedAmount,
};

const FEE_RATE_DENOMINATOR: u64 = 100_000;

pub fn check_stopped(config: &Config) -> Result<(), ContractError> {
    if config.stopped {
        return Err(ContractError::Stopped {});
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
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    // NOTE: In the original milkyway implementation, this was optional and defaulted to info.sender if not set. Due to how we will be using this contract (i.e. with the funded-dispatch contract in the zkgm batch), it is much clearer to enforce this to be set. This makes it slightly more verbose for a user who is liquid staking directly on union, but we deem this to be a bit of a degenerate case as this will only be done in the ui from ethereum (via zkgm).
    mint_to: Bytes,
    recipient_channel_id: Option<ChannelId>,
    min_mint_amount: Option<Uint128>,
) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    check_stopped(&config)?;

    // NOTE: This used to default to info.sender if not provided (see comment on mint_to parameter above)
    let mint_to_address = mint_to;

    let mut state: State = STATE.load(deps.storage)?;
    ensure!(
        amount >= config.minimum_liquid_stake_amount,
        ContractError::MinimumLiquidStakeAmount {
            minimum_stake_amount: config.minimum_liquid_stake_amount,
            sent_amount: amount,
        }
    );

    // TODO: Review if this is still necessary now that the staking is happening on the same chain
    // this handles a special case that through slashing and redeeming chaining we get into a state
    // where the total liquid stake is zero but the total native stake is not
    // nobody can claim the native stake, so we need to claim it to the DAO
    if state.total_liquid_stake_token.is_zero() && !state.total_native_token.is_zero() {
        state.total_fees += state.total_native_token;
        state.total_native_token = Uint128::zero();
    }

    // Compute mint amount
    let mint_amount = compute_mint_amount(
        state.total_native_token,
        state.total_liquid_stake_token,
        amount,
    );

    // If mint amount is zero it is likely there was a an issue with rounding, return error and do not mint
    if mint_amount.is_zero() {
        return Err(ContractError::MintError {});
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

    // Mint liquid staking token to self
    let mint_lst_msg = wasm_execute(
        &config.liquid_stake_token_address,
        &Cw20ExecuteMsg::Mint {
            amount: mint_amount,
            recipient: env.contract.address.to_string(),
        },
        vec![],
    )?;

    // Transfer native token to multisig address
    // NOTE: In the original milkyway satking contracts, this was an ibc transfer message back to the multisig on the source chain since the liquid staking is *initiated* on the protocol chain (i.e. where this contract is deployed), but the *staking* happens on the native chain
    // TODO: This should be a wasm_execute call with a specific ExecuteMsg for the staker contract
    // TODO: Define the ExecuteMsg for the staker contract
    let transfer_funds_to_cw_account_msig_message = BankMsg::Send {
        to_address: config.staker_address.to_string(),
        // TODO: Ensure only the staking token is provided here
        amount: info.funds,
    };
    state.total_native_token += amount;
    state.total_liquid_stake_token += mint_amount;
    STATE.save(deps.storage, &state)?;

    let response = Response::new()
        .add_message(mint_lst_msg)
        .add_message(transfer_funds_to_cw_account_msig_message)
        .add_attribute("mint_to_address", mint_to_address.to_string())
        .add_attribute("action", "liquid_stake")
        // NOTE: In practice, this will always be the funded-dispatch contract. This may need to be changed to emit the original sender from the source chain (if it exists).
        .add_attribute("sender", info.sender.to_string())
        .add_attribute("in_amount", amount)
        .add_attribute("mint_amount", mint_amount);

    // NOTE: This used to be a check against `mint_to_is_protocol` (see commented out code above) with the branches flipped.
    let response = if let Some(recipient_channel_id) = recipient_channel_id {
        // transfer the eU to the desired recipient channel
        response.add_messages([
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
                        env.block.time.plus_days(3).nanos(),
                    ),
                    timeout_height: 0_u64.into(),
                    // salt is empty as the timestamp will make the packet unique
                    salt: Default::default(),
                    instruction: Instruction {
                        version: INSTR_VERSION_2,
                        opcode: OP_TOKEN_ORDER,
                        operand: TokenOrderV2 {
                            sender: env.contract.address.as_bytes().to_vec().into(),
                            receiver: mint_to_address.into(),
                            base_token: config
                                .liquid_stake_token_address
                                .clone()
                                .into_bytes()
                                .into(),
                            base_amount: alloy::primitives::U256::from(mint_amount.u128()),

                            // these depend on the channel, we need to either query or configure fungible counterparties
                            quote_token: todo!(),
                            quote_amount: alloy::primitives::U256::from(mint_amount.u128()),
                            kind: TOKEN_ORDER_KIND_ESCROW,
                            // TODO: What does this need to be?
                            metadata: Default::default(),
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
            &Cw20ExecuteMsg::Transfer {
                amount: mint_amount,
                recipient: recipient.to_string(),
            },
            vec![],
        )?)
    };

    Ok(response)
}

/// # Changes from original implementation
///
/// - added `sender` parameter and check against info.sender and config.funded_dispatch_address
pub fn execute_unbond(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
    staker: String,
) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    check_stopped(&config)?;

    if info.sender.as_str() != staker {
        // funded_dispatch_address is a trusted address
        if info.sender != config.funded_dispatch_address {
            panic!("nice try fucker")
        }
    }

    STATE.load(deps.storage)?;

    // Load current pending batch
    let pending_batch_id = PENDING_BATCH_ID.load(deps.storage)?;

    // NOTE: This is just a convoluted `.has()` check
    // Add unstake request to pending batch
    let pending_unstake_request =
        unstake_requests().may_load(deps.storage, (pending_batch_id, staker.clone()))?;
    let is_new_request = pending_unstake_request.is_none();
    match pending_unstake_request {
        Some(_) => {
            unstake_requests().update(
                deps.storage,
                (pending_batch_id, staker.clone()),
                |or| -> Result<UnstakeRequest, ContractError> {
                    match or {
                        Some(r) => Ok(UnstakeRequest {
                            batch_id: r.batch_id,
                            user: r.user.clone(),
                            amount: r.amount + amount,
                        }),
                        None => Err(ContractError::NoRequestInBatch {}),
                    }
                },
            )?;
        }
        None => {
            new_unstake_request(&mut deps, staker.clone(), pending_batch_id, amount)?;
        }
    }

    // Add amount to batch total (stTIA)
    BATCHES.update(
        deps.storage,
        pending_batch_id,
        |batch| -> Result<Batch, ContractError> {
            let mut batch = batch.unwrap();
            batch.batch_total_liquid_stake += amount;
            if is_new_request {
                batch.unstake_requests_count = Some(batch.unstake_requests_count.unwrap_or(0) + 1);
            }
            Ok(batch)
        },
    )?;

    // burn `amount` of lst from the staker
    // this requires an allowance to spend these tokens on behalf of the staker
    let lst_burn_msg = wasm_execute(
        &config.liquid_stake_token_address,
        &Cw20ExecuteMsg::BurnFrom {
            owner: staker.clone(),
            amount,
        },
        vec![],
    )?;

    Ok(Response::new()
        .add_message(lst_burn_msg)
        .add_attribute("action", "liquid_unstake")
        .add_attribute("sender", staker)
        .add_attribute("batch", pending_batch_id.to_string())
        .add_attribute("amount", amount))
}

/// Submit batch and transition pending batch to submitted.
///
/// # Changes from original implementation
///
/// - oracle functionality was removed
/// TODO: Withdraw unstaked tokens in this function
pub fn execute_submit_batch(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    check_stopped(&config)?;

    let pending_batch_id = PENDING_BATCH_ID.load(deps.storage)?;
    let mut batch = BATCHES.load(deps.storage, pending_batch_id)?;

    if let Some(est_next_batch_time) = batch.next_batch_action_time {
        // Check if the batch has been submitted
        if env.block.time.seconds() < est_next_batch_time {
            return Err(ContractError::BatchNotReady {
                actual: env.block.time.seconds(),
                expected: est_next_batch_time,
            });
        }
    } else {
        // Should not enter as pending batch should have a next batch action time
        return Err(ContractError::BatchNotReady {
            actual: env.block.time.seconds(),
            expected: 0u64,
        });
    }

    if batch.unstake_requests_count.unwrap_or(0) == 0 {
        return Err(ContractError::BatchEmpty {});
    }

    let mut state = STATE.load(deps.storage)?;

    ensure!(
        state.total_liquid_stake_token >= batch.batch_total_liquid_stake,
        ContractError::InvalidUnstakeAmount {
            total_liquid_stake_token: state.total_liquid_stake_token,
            amount_to_unstake: batch.batch_total_liquid_stake
        }
    );

    // Create new pending batch
    let new_pending_batch = Batch::new(
        batch.id + 1,
        Uint128::zero(),
        env.block.time.seconds() + config.batch_period,
    );

    // Save new pending batch
    BATCHES.save(deps.storage, new_pending_batch.id, &new_pending_batch)?;
    PENDING_BATCH_ID.save(deps.storage, &new_pending_batch.id)?;

    // Waiting until batch submission to burn tokens
    let lst_burn_msg = wasm_execute(
        config.liquid_stake_token_address,
        &Cw20ExecuteMsg::Burn {
            amount: batch.batch_total_liquid_stake,
        },
        vec![],
    )?;

    let unbond_amount = compute_unbond_amount(
        state.total_native_token,
        state.total_liquid_stake_token,
        batch.batch_total_liquid_stake,
    );

    // Reduce underlying TIA balance by unbonded amount
    state.total_native_token = state
        .total_native_token
        .checked_sub(unbond_amount)
        .unwrap_or_else(|_| Uint128::zero());

    // Reduce underlying stTIA balance by batch total
    state.total_liquid_stake_token = state
        .total_liquid_stake_token
        .checked_sub(batch.batch_total_liquid_stake)
        .unwrap_or_else(|_| Uint128::zero());

    STATE.save(deps.storage, &state)?;

    // Update batch status
    batch.expected_native_unstaked = Some(unbond_amount);
    batch.update_status(
        BatchStatus::Submitted,
        Some(env.block.time.seconds() + config.unbonding_period),
    );

    BATCHES.save(deps.storage, batch.id, &batch)?;

    Ok(Response::new()
        .add_message(lst_burn_msg)
        .add_attribute("action", "submit_batch")
        .add_attribute("batch_id", batch.id.to_string())
        .add_attribute("batch_total", batch.batch_total_liquid_stake)
        .add_attribute("expected_native_unstaked", unbond_amount))
}

// doing a "push over pool" pattern for now
// eventually we can move this to auto-withdraw all funds upon batch completion
// Reasoning - any one issue in the batch will cause the entire batch to fail
/// # Changes from original implementation
///
/// - oracle functionality was removed
/// - added `sender` parameter and check against info.sender and config.funded_dispatch_address
pub fn execute_withdraw(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    batch_id: u64,
    sender: String,
) -> ContractResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    check_stopped(&config)?;

    if info.sender.as_str() != sender {
        // funded_dispatch_address is a trusted address
        if info.sender != config.funded_dispatch_address {
            panic!("nice try fucker")
        }
    }

    // nice
    let _batch = BATCHES.load(deps.storage, batch_id);
    if _batch.is_err() {
        return Err(ContractError::BatchEmpty {});
    }
    let batch = _batch.unwrap();

    if batch.status != BatchStatus::Received {
        return Err(ContractError::TokensAlreadyClaimed { batch_id: batch.id });
    }
    // TODO: Why unwrap?
    let received_native_unstaked = batch.received_native_unstaked.as_ref().unwrap();

    let liquid_unstake_request = unstake_requests()
        .may_load(deps.storage, (batch.id, sender.clone()))?
        .ok_or(ContractError::NoRequestInBatch {})?;

    let amount = received_native_unstaked.multiply_ratio(
        liquid_unstake_request.amount,
        batch.batch_total_liquid_stake,
    );

    remove_unstake_request(&mut deps, sender.clone(), batch.id)?;

    // send the native token (U) back to the staker
    let send_msg = BankMsg::Send {
        to_address: sender.clone(),
        amount: vec![Coin {
            denom: config.native_token_denom.clone(),
            amount,
        }],
    };

    Ok(Response::new()
        .add_message(send_msg)
        .add_attribute("action", "execute_withdraw")
        .add_attribute("batch", batch.id.to_string())
        .add_attribute("amount", amount.to_string()))
}

// TODO: Implement once basic functionality is completed
// // Add a validator to the list of validators; callable by the owner
// pub fn execute_add_validator(
//     deps: DepsMut,
//     _env: Env,
//     info: MessageInfo,
//     new_validator: String,
// ) -> ContractResult<Response> {
//     ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

//     let mut config = CONFIG.load(deps.storage)?;
//     let new_validator_addr = validate_address(&new_validator, &config.validator_address_prefix)?;

//     // Check if the new_validator is already in the list.
//     if config
//         .validators
//         .iter()
//         .any(|validator| *validator == new_validator_addr)
//     {
//         return Err(ContractError::DuplicateValidator {
//             validator: new_validator.clone(),
//         });
//     }

//     // Add the new validator to the list.
//     config.validators.push(new_validator_addr.clone());

//     // Save the updated config.
//     CONFIG.save(deps.storage, &config)?;

//     Ok(Response::new()
//         .add_attribute("action", "add_validator")
//         .add_attribute("new_validator", new_validator_addr)
//         .add_attribute("sender", info.sender))
// }

// TODO: Implement once basic functionality is completed
// pub fn execute_remove_validator(
//     deps: DepsMut,
//     _env: Env,
//     info: MessageInfo,
//     validator_to_remove: String,
// ) -> ContractResult<Response> {
//     ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

//     let mut config = CONFIG.load(deps.storage)?;
//     let validator_addr_to_remove =
//         validate_address(&validator_to_remove, &config.validator_address_prefix)?;

//     // Find the position of the validator to be removed.
//     if let Some(pos) = config
//         .validators
//         .iter()
//         .position(|validator| *validator == validator_addr_to_remove)
//     {
//         // Remove the validator if found.
//         config.validators.remove(pos);
//     } else {
//         // If the validator is not found, return an error.
//         return Err(ContractError::ValidatorNotFound {
//             validator: validator_to_remove.clone(),
//         });
//     }

//     // Save the updated config.
//     CONFIG.save(deps.storage, &config)?;

//     Ok(Response::new()
//         .add_attribute("action", "remove_validator")
//         .add_attribute("removed_validator", validator_addr_to_remove)
//         .add_attribute("sender", info.sender))
// }

// Transfer ownership to another account; callable by the owner
// This will require the new owner to accept to take effect.
// No need to handle case of overwriting the pending owner
// Ownership can only be claimed after 7 days to mitigate fat finger errors
pub fn execute_transfer_ownership(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: String,
) -> ContractResult<Response> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let mut state: State = STATE.load(deps.storage)?;
    state.pending_owner = Some(deps.api.addr_validate(&new_owner)?);
    state.owner_transfer_min_time = Some(Timestamp::from_seconds(
        _env.block.time.seconds() + 60 * 60 * 24 * 7,
    )); // 7 days

    STATE.save(deps.storage, &state)?;

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

    let mut state = STATE.load(deps.storage)?;
    state.pending_owner = None;
    state.owner_transfer_min_time = None;

    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attribute("action", "revoke_ownership_transfer"))
}

pub fn execute_accept_ownership(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> ContractResult<Response> {
    let mut state: State = STATE.load(deps.storage)?;
    if state.owner_transfer_min_time.is_some()
        && state.owner_transfer_min_time.unwrap().seconds() > _env.block.time.seconds()
    {
        return Err(ContractError::OwnershipTransferNotReady {
            time_to_claim: Timestamp::from_seconds(
                state.owner_transfer_min_time.unwrap().seconds(),
            ),
        });
    }

    let new_owner = {
        match state.pending_owner {
            Some(pending_owner) if pending_owner == info.sender => {
                state.pending_owner = None;
                STATE.save(deps.storage, &state)?;
                Some(pending_owner)
            }
            _ => None,
        }
    };

    match new_owner {
        Some(pending_owner) => {
            ADMIN.set(deps, Some(pending_owner))?;
            Ok(Response::new()
                .add_attribute("action", "accept_ownership")
                .add_attribute("new_owner", info.sender))
        }
        None => Err(ContractError::NoPendingOwner {}),
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

//     let mut config: Config = CONFIG.load(deps.storage)?;

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

/// # Changes from original implementation
///
/// - oracle functionality removed
/// - added `amount` parameter since the token is now a cw20 and the amount cannot be inferred from the `info.funds`
///
/// NOTE: This business logic is still needed for updating the state of the lst contract, but it will not be in this entrypoint (receive_rewards will be removed as an entrypoint)
pub fn receive_rewards(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> ContractResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;
    let mut state: State = STATE.load(deps.storage)?;

    check_stopped(&config)?;

    if state.total_liquid_stake_token.is_zero() {
        return Err(ContractError::NoLiquidStake {});
    }

    let coin = info
        .funds
        .iter()
        .find(|c| c.denom == config.native_token_denom);
    // nice is_none/unwrap
    if coin.is_none() {
        return Err(ContractError::Payment(PaymentError::NoFunds {}));
    }

    let amount = coin.unwrap().amount;
    let fee = config
        .protocol_fee_config
        .fee_rate
        .multiply_ratio(amount, FEE_RATE_DENOMINATOR);
    if fee.is_zero() {
        return Err(ContractError::ComputedFeesAreZero {
            received_rewards: amount,
        });
    }
    let amount_after_fees = amount.checked_sub(fee);
    if amount_after_fees.is_err() {
        return Err(ContractError::ReceiveRewardsTooSmall {
            amount,
            minimum: fee,
        });
    }
    let amount_after_fees = amount_after_fees.unwrap();

    // update the accounting of tokens
    state.total_native_token += amount_after_fees;
    state.total_reward_amount += amount;
    if config.protocol_fee_config.fee_recipient.is_none() {
        state.total_fees += fee;
    }

    STATE.save(deps.storage, &state)?;

    // transfer the funds to Celestia to be staked
    // TODO: Figure out wtf to do here lol
    let ibc_transfer_msg = ibc_transfer_sub_msg(
        &mut deps,
        &env,
        &config.staker_address,
        Coin::new(amount_after_fees.u128(), &config.native_token_denom),
        None,
    )?;

    let mut response = Response::new()
        .add_attribute("action", "receive_rewards")
        .add_attribute("action", "transfer_stake")
        .add_attribute("amount", amount)
        .add_attribute("amount_after_fees", amount_after_fees)
        .add_submessage(ibc_transfer_msg);

    if let Some(fee_recipient) = config.protocol_fee_config.fee_recipient {
        response = response.add_message(cosmwasm_std::BankMsg::Send {
            to_address: fee_recipient.to_string(),
            amount: vec![cosmwasm_std::Coin::new(
                fee.u128(),
                config.native_token_denom,
            )],
        });
    }

    Ok(response)
}

/// Marks a batch as received
/// Public function? Permissionless?
pub fn receive_unstaked_tokens(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    batch_id: u64,
) -> ContractResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    check_stopped(&config)?;

    let coin = info
        .funds
        .iter()
        .find(|c| c.denom == config.native_token_denom);
    if coin.is_none() {
        return Err(ContractError::Payment(PaymentError::NoFunds {}));
    }

    let amount = coin.unwrap().amount;

    let mut batch: Batch = BATCHES.load(deps.storage, batch_id)?;

    if batch.status != BatchStatus::Submitted {
        return Err(ContractError::BatchNotClaimable {
            batch_id: batch.id,
            status: batch.status,
        });
    }

    if batch.next_batch_action_time.is_none() {
        return Err(ContractError::BatchNotClaimable {
            batch_id: batch.id,
            status: batch.status,
        });
    }
    let next_batch_action_time = batch.next_batch_action_time.unwrap();
    if next_batch_action_time > env.block.time.seconds() {
        return Err(ContractError::BatchNotReady {
            actual: env.block.time.seconds(),
            expected: next_batch_action_time,
        });
    }

    let expected_native_amount = batch
        .expected_native_unstaked
        .ok_or(ContractError::BatchWithoutExpectedNativeAmount { batch_id })?;
    if expected_native_amount != amount {
        return Err(ContractError::ReceivedWrongBatchAmount {
            batch_id,
            expected: expected_native_amount,
            received: amount,
        });
    }

    batch.received_native_unstaked = Some(amount);
    batch.update_status(BatchStatus::Received, None);

    BATCHES.save(deps.storage, batch.id, &batch)?;

    Ok(Response::new()
        .add_attribute("action", "receive_unstaked_tokens")
        .add_attribute("batch", batch_id.to_string())
        .add_attribute("amount", amount))
}

pub fn circuit_breaker(deps: DepsMut, _env: Env, info: MessageInfo) -> ContractResult<Response> {
    let sender = info.sender.to_string();

    let mut config: Config = CONFIG.load(deps.storage)?;

    if ADMIN.assert_admin(deps.as_ref(), &info.sender).is_err()
        && !config.monitors.iter().any(|v| *v == sender)
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

    let mut config: Config = CONFIG.load(deps.storage)?;
    if !config.stopped {
        return Err(ContractError::NotStopped {});
    }

    config.stopped = false;
    CONFIG.save(deps.storage, &config)?;

    STATE.update(
        deps.storage,
        |mut state| -> Result<State, cosmwasm_std::StdError> {
            state.total_native_token = total_native_token;
            state.total_liquid_stake_token = total_liquid_stake_token;
            state.total_reward_amount = total_reward_amount;
            Ok(state)
        },
    )?;

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
        return Err(ContractError::NotStopped {});
    }

    for batch_expected_amount in expected_amounts.iter() {
        let mut batch = BATCHES.load(deps.storage, batch_expected_amount.batch_id)?;
        if batch.status != BatchStatus::Pending && batch.status != BatchStatus::Submitted {
            return Err(ContractError::UnexpecedBatchStatus {
                actual: batch.status,
            });
        }

        if batch.expected_native_unstaked.is_none() {
            return Err(ContractError::BatchWithoutExpectedNativeAmount {
                batch_id: batch_expected_amount.batch_id,
            });
        };

        batch.expected_native_unstaked = Some(batch_expected_amount.amount);

        BATCHES.save(deps.storage, batch_expected_amount.batch_id, &batch)?;
    }

    Ok(Response::new()
        .add_attribute("action", "slash_batches")
        .add_attribute("updated_batches", serde_json::to_string(&expected_amounts)?))
}

// Why not send the fees to the fee_recipient directly when receiving the rewards?
pub fn fee_withdraw(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> ContractResult<Response> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let config: Config = CONFIG.load(deps.storage)?;
    let mut state: State = STATE.load(deps.storage)?;

    if state.total_fees < amount {
        return Err(ContractError::InsufficientFunds {});
    }

    if config.protocol_fee_config.fee_recipient.is_none() {
        return Err(ContractError::TreasuryNotConfigured {});
    }
    let fee_recipient = config
        .protocol_fee_config
        .fee_recipient
        .unwrap()
        .to_string();

    state.total_fees = state.total_fees.checked_sub(amount).unwrap();
    STATE.save(deps.storage, &state)?;

    let send_msg = BankMsg::Send {
        to_address: fee_recipient.clone(),
        amount: vec![Coin {
            denom: config.native_token_denom,
            amount,
        }],
    };

    Ok(Response::new()
        .add_attribute("action", "fee_withdraw")
        .add_attribute("receiver", fee_recipient)
        .add_attribute("amount", amount)
        .add_message(send_msg))
}
