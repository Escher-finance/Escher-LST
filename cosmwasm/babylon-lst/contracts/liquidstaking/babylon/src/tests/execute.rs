use std::str::FromStr;

use cosmwasm_std::{
    Binary, Coin, Decimal, Uint128,
    testing::{message_info, mock_dependencies, mock_env},
    to_json_binary,
};
use cw20::AllowanceResponse;

use crate::{
    ContractError,
    execute::*,
    msg::Recipient,
    query::{query_bond_chains, query_unbond_chains},
    state::{PARAMETERS, QuoteToken, STATE, STATUS, Status},
    tests::{mock_parameters, mock_state},
    utils,
};

#[test]
fn test_calculate_native_token() {
    let staking_token = Uint128::from(10000u32);

    let exchange_rate = Decimal::from_ratio(
        Uint128::from(5_350_444_044_771_u128),
        Uint128::from(30000u128),
    );

    println!("exchange_rate: {exchange_rate}");

    let undelegate_amount: Uint128 =
        utils::calc::calculate_native_token_from_staking_token(staking_token, exchange_rate);
    println!("undelegate_amount: {undelegate_amount}");
}

#[test]
fn exchange_rate_calculation() {
    let total_bond = Uint128::new(100);

    let a = Uint128::new(10);
    let b = Uint128::new(50);
    let exchange_rate = Decimal::from_ratio(a, b);
    println!("{total_bond:?} / {exchange_rate:?}");

    let token = utils::calc::calculate_staking_token_from_rate(total_bond, exchange_rate);

    println!("token: {token:?}");
    assert_eq!(token, Uint128::new(500));

    // - Rewards for 4 days: 1000 Union * 0.0274% * 4 = 1.096 Union
    // - Total staked Union + rewards (U + R): 1001.096 Union
    // - Total LUnion (L): 1000 LUnion

    // - New exchange rate: 1001.096 / 1000 = 1.001096 Union per LUnion
    // - Bob receives: 500 / 1.001096 = 499.45 LUnion

    let a = Uint128::new(1_001_096);
    let b = Uint128::new(1_000_000);
    let new_exchange_rate = Decimal::from_ratio(a, b);

    let bond_amount = Uint128::new(500_000_000);
    let mint_amount =
        utils::calc::calculate_staking_token_from_rate(bond_amount, new_exchange_rate);
    assert_eq!(mint_amount, Uint128::new(499_452_599));
    println!("mint_amount: {mint_amount:?}");
}

#[test]
fn exchange_unbond_rate_calculation() {
    let staking_token = Uint128::new(100);

    let a = Uint128::new(110);
    let b = Uint128::new(100);
    let exchange_rate = Decimal::from_ratio(a, b);

    let token =
        utils::calc::calculate_native_token_from_staking_token(staking_token, exchange_rate);
    assert_eq!(token, Uint128::new(110));
}

#[test]
fn slippage_calculation() {
    let expected = Uint128::new(10000);
    let slippage = Decimal::from_str("0.01").unwrap();
    let output = Uint128::new(10140);

    let result = utils::calc::check_slippage(output, expected, slippage);
    assert!(result.is_err());

    let output = Uint128::new(10100);
    let result = utils::calc::check_slippage(output, expected, slippage);
    assert!(result.is_ok());
}

#[test]
fn test_update_quote_token_channel_id_should_match() {
    use cosmwasm_std::testing::{mock_dependencies, mock_env};

    let mut deps = mock_dependencies();
    let env = mock_env();

    let owner = deps.api.addr_make("owner");

    let info = cosmwasm_std::MessageInfo {
        sender: owner.clone(),
        funds: vec![],
    };
    let mut channel_id = 10;
    let quote_token = QuoteToken {
        channel_id,
        quote_token: "a".to_string(),
        lst_quote_token: "b".to_string(),
    };

    cw_ownable::initialize_owner(&mut deps.storage, &deps.api, Some(owner.as_str())).unwrap();

    // Good
    update_quote_token(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        channel_id,
        quote_token.clone(),
    )
    .unwrap();

    channel_id += 1;

    // Fails - channel_id doesn't match
    let err = update_quote_token(deps.as_mut(), env, info, channel_id, quote_token).unwrap_err();
    assert!(matches!(err, ContractError::InvalidQuoteTokens {}));
}

#[test]
fn test_slash_batch() {
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};

    use crate::{
        contract::execute,
        error::ContractError,
        msg::{BatchReceivedAmount, ExecuteMsg},
        utils::batch::{Batch, BatchStatus, batches},
    };

    let mut deps = mock_dependencies();
    let mut env = mock_env();

    // Setup owner
    let owner = deps.api.addr_make("owner");
    let non_owner = deps.api.addr_make("non_owner");
    cw_ownable::initialize_owner(&mut deps.storage, &deps.api, Some(owner.as_str())).unwrap();

    // Test 1: Only owner can access
    let info = message_info(&non_owner, &[]);
    let msg = ExecuteMsg::SlashBatch {
        new_received_amounts: vec![],
    };
    let err = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    assert!(matches!(err, ContractError::Ownership(_)));

    // Test 2: Only batch with submitted status can be slashed
    // Create a batch with Pending status
    let batch_id_pending = 1u64;
    let batch_pending = Batch {
        id: batch_id_pending,
        total_liquid_stake: cosmwasm_std::Uint128::new(1000),
        expected_native_unstaked: Some(cosmwasm_std::Uint128::new(900)),
        received_native_unstaked: None,
        unbond_records_count: 5,
        next_batch_action_time: Some(env.block.time.seconds() + 1000),
        status: BatchStatus::Pending,
    };
    batches()
        .save(deps.as_mut().storage, batch_id_pending, &batch_pending)
        .unwrap();

    let info = message_info(&owner, &[]);
    let msg = ExecuteMsg::SlashBatch {
        new_received_amounts: vec![BatchReceivedAmount {
            id: batch_id_pending,
            received: cosmwasm_std::Uint128::new(800),
        }],
    };
    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::BatchStatusIncorrect {
            actual: _,
            expected: _
        }
    ));

    // Test 3: Batch not ready - block time is before next_batch_action_time
    let batch_id_not_ready = 7u64;
    let future_time = env.block.time.seconds() + 10000;
    let batch_not_ready = Batch {
        id: batch_id_not_ready,
        total_liquid_stake: cosmwasm_std::Uint128::new(1000),
        expected_native_unstaked: Some(cosmwasm_std::Uint128::new(900)),
        received_native_unstaked: None,
        unbond_records_count: 5,
        next_batch_action_time: Some(future_time),
        status: BatchStatus::Submitted,
    };
    batches()
        .save(deps.as_mut().storage, batch_id_not_ready, &batch_not_ready)
        .unwrap();

    let msg = ExecuteMsg::SlashBatch {
        new_received_amounts: vec![BatchReceivedAmount {
            id: batch_id_not_ready,
            received: cosmwasm_std::Uint128::new(800),
        }],
    };
    let not_ready_err = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(not_ready_err.is_err());

    // Test 4: Batch without next_batch_action_time should fail
    let batch_id_no_action_time = 8u64;
    let batch_no_action_time = Batch {
        id: batch_id_no_action_time,
        total_liquid_stake: cosmwasm_std::Uint128::new(1000),
        expected_native_unstaked: Some(cosmwasm_std::Uint128::new(900)),
        received_native_unstaked: None,
        unbond_records_count: 5,
        next_batch_action_time: None, // No action time set
        status: BatchStatus::Submitted,
    };
    batches()
        .save(
            deps.as_mut().storage,
            batch_id_no_action_time,
            &batch_no_action_time,
        )
        .unwrap();

    let msg = ExecuteMsg::SlashBatch {
        new_received_amounts: vec![BatchReceivedAmount {
            id: batch_id_no_action_time,
            received: cosmwasm_std::Uint128::new(800),
        }],
    };
    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert!(matches!(err, ContractError::BatchNextActionTimeNotSet));

    // Test 5: Create batch with Submitted status and test successful slash
    // Set env time to be after the batch action time
    let batch_id_submitted = 2u64;
    let expected_amount = cosmwasm_std::Uint128::new(1000);
    let received_amount = cosmwasm_std::Uint128::new(800);
    let batch_action_time = env.block.time.seconds() + 100;
    let batch_submitted = Batch {
        id: batch_id_submitted,
        total_liquid_stake: cosmwasm_std::Uint128::new(1100),
        expected_native_unstaked: Some(expected_amount),
        received_native_unstaked: None,
        unbond_records_count: 10,
        next_batch_action_time: Some(batch_action_time),
        status: BatchStatus::Submitted,
    };
    batches()
        .save(deps.as_mut().storage, batch_id_submitted, &batch_submitted)
        .unwrap();

    // Move time forward to be after batch_action_time
    env.block.time = env.block.time.plus_seconds(200);

    let msg = ExecuteMsg::SlashBatch {
        new_received_amounts: vec![BatchReceivedAmount {
            id: batch_id_submitted,
            received: received_amount,
        }],
    };

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Verify response has events
    assert_eq!(res.events.len(), 2); // BatchReceivedEvent + slash_batch event

    // Verify batch status changed to Received
    let updated_batch = batches()
        .load(deps.as_ref().storage, batch_id_submitted)
        .unwrap();
    assert!(matches!(updated_batch.status, BatchStatus::Received));
    assert_eq!(
        updated_batch.received_native_unstaked,
        Some(received_amount)
    );

    // Test 6: Check received amount not over the expected native unstaked amount
    let batch_id_over = 3u64;
    let expected_amount_3 = cosmwasm_std::Uint128::new(1000);
    let received_amount_over = cosmwasm_std::Uint128::new(1001); // Over expected
    let batch_over = Batch {
        id: batch_id_over,
        total_liquid_stake: cosmwasm_std::Uint128::new(1100),
        expected_native_unstaked: Some(expected_amount_3),
        received_native_unstaked: None,
        unbond_records_count: 8,
        next_batch_action_time: Some(env.block.time.seconds() - 100), // In the past
        status: BatchStatus::Submitted,
    };
    batches()
        .save(deps.as_mut().storage, batch_id_over, &batch_over)
        .unwrap();

    let msg = ExecuteMsg::SlashBatch {
        new_received_amounts: vec![BatchReceivedAmount {
            id: batch_id_over,
            received: received_amount_over,
        }],
    };
    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::SlashBatchReceivedAmountExceedExpected { .. }
    ));

    // Test 7: Multiple batches can be slashed at once and received amounts set correctly
    let batch_id_multi_1 = 4u64;
    let batch_id_multi_2 = 5u64;

    let batch_multi_1 = Batch {
        id: batch_id_multi_1,
        total_liquid_stake: cosmwasm_std::Uint128::new(2000),
        expected_native_unstaked: Some(cosmwasm_std::Uint128::new(1800)),
        received_native_unstaked: None,
        unbond_records_count: 15,
        next_batch_action_time: Some(env.block.time.seconds() - 1000), // In the past
        status: BatchStatus::Submitted,
    };

    let batch_multi_2 = Batch {
        id: batch_id_multi_2,
        total_liquid_stake: cosmwasm_std::Uint128::new(3000),
        expected_native_unstaked: Some(cosmwasm_std::Uint128::new(2700)),
        received_native_unstaked: None,
        unbond_records_count: 20,
        next_batch_action_time: Some(env.block.time.seconds() - 500), // In the past
        status: BatchStatus::Submitted,
    };

    batches()
        .save(deps.as_mut().storage, batch_id_multi_1, &batch_multi_1)
        .unwrap();
    batches()
        .save(deps.as_mut().storage, batch_id_multi_2, &batch_multi_2)
        .unwrap();

    let received_multi_1 = cosmwasm_std::Uint128::new(1500);
    let received_multi_2 = cosmwasm_std::Uint128::new(2500);

    let msg = ExecuteMsg::SlashBatch {
        new_received_amounts: vec![
            BatchReceivedAmount {
                id: batch_id_multi_1,
                received: received_multi_1,
            },
            BatchReceivedAmount {
                id: batch_id_multi_2,
                received: received_multi_2,
            },
        ],
    };
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Verify events (2 events per batch: BatchReceivedEvent + slash_batch)
    assert_eq!(res.events.len(), 4);

    // Verify both batches updated correctly
    let updated_batch_1 = batches()
        .load(deps.as_ref().storage, batch_id_multi_1)
        .unwrap();
    assert!(matches!(updated_batch_1.status, BatchStatus::Received));
    assert_eq!(
        updated_batch_1.received_native_unstaked,
        Some(received_multi_1)
    );

    let updated_batch_2 = batches()
        .load(deps.as_ref().storage, batch_id_multi_2)
        .unwrap();
    assert!(matches!(updated_batch_2.status, BatchStatus::Received));
    assert_eq!(
        updated_batch_2.received_native_unstaked,
        Some(received_multi_2)
    );

    // Test 8: Batch without expected_native_unstaked should fail
    let batch_id_no_expected = 6u64;
    let batch_no_expected = Batch {
        id: batch_id_no_expected,
        total_liquid_stake: cosmwasm_std::Uint128::new(1000),
        expected_native_unstaked: None, // No expected amount set
        received_native_unstaked: None,
        unbond_records_count: 5,
        next_batch_action_time: Some(env.block.time.seconds() - 100), // In the past
        status: BatchStatus::Submitted,
    };
    batches()
        .save(
            deps.as_mut().storage,
            batch_id_no_expected,
            &batch_no_expected,
        )
        .unwrap();

    let info = message_info(&owner, &[]);
    let msg = ExecuteMsg::SlashBatch {
        new_received_amounts: vec![BatchReceivedAmount {
            id: batch_id_no_expected,
            received: cosmwasm_std::Uint128::new(500),
        }],
    };
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::BatchExpectedNativeUnstakedNotSet
    ));
}

#[test]
fn test_bond_must_fail_if_paused() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    let status = Status {
        bond_is_paused: true,
        unbond_is_paused: false,
    };
    STATUS.save(deps.as_mut().storage, &status).unwrap();

    let err = bond(
        deps.as_mut(),
        env,
        info,
        None,
        Uint128::one(),
        sender.clone(),
    )
    .unwrap_err();

    assert!(matches!(
        err,
        ContractError::FunctionalityUnderMaintenance {}
    ))
}

#[test]
fn test_bond_must_fail_if_invalid_funds() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();

    let sender = api.addr_make("sender");

    let denom = "denom".to_string();

    let amount = Uint128::new(1000);
    let info = message_info(&sender, &[Coin::new(amount, denom.clone())]);

    let status = Status {
        bond_is_paused: false,
        unbond_is_paused: false,
    };
    STATUS.save(deps.as_mut().storage, &status).unwrap();

    let mut params = mock_parameters();
    params.underlying_coin_denom = denom.clone();
    params.min_bond = amount + Uint128::one();
    PARAMETERS.save(deps.as_mut().storage, &params).unwrap();

    let err = bond(
        deps.as_mut(),
        env,
        info,
        None,
        Uint128::one(),
        sender.clone(),
    )
    .unwrap_err();

    assert!(matches!(err, ContractError::BondAmountTooLow {}))
}

#[test]
fn test_unbond_must_fail_if_funds_are_attached() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();

    let sender = api.addr_make("sender");

    let denom = "denom".to_string();

    let amount = Uint128::new(1000);
    let info = message_info(&sender, &[Coin::new(amount, denom.clone())]);

    let recipient = Recipient::OnChain {
        address: sender.clone(),
    };

    let err = unbond(deps.as_mut(), env, info, amount, recipient).unwrap_err();

    assert!(matches!(
        err,
        ContractError::Payment(cw_utils::PaymentError::NonPayable {})
    ))
}

#[test]
fn test_unbond_must_fail_if_paused() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();

    let sender = api.addr_make("sender");

    let amount = Uint128::new(1000);
    let info = message_info(&sender, &[]);

    let recipient = Recipient::OnChain {
        address: sender.clone(),
    };

    let status = Status {
        bond_is_paused: false,
        unbond_is_paused: true,
    };
    STATUS.save(deps.as_mut().storage, &status).unwrap();

    let err = unbond(deps.as_mut(), env, info, amount, recipient).unwrap_err();

    assert!(matches!(
        err,
        ContractError::FunctionalityUnderMaintenance {},
    ))
}

#[test]
fn test_unbond_must_fail_if_invalid_exchange_rate() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();

    let sender = api.addr_make("sender");

    let amount = Uint128::new(1000);
    let info = message_info(&sender, &[]);

    let recipient = Recipient::OnChain {
        address: sender.clone(),
    };

    let status = Status {
        bond_is_paused: false,
        unbond_is_paused: false,
    };
    STATUS.save(deps.as_mut().storage, &status).unwrap();

    let mut state = mock_state();
    state.exchange_rate = Decimal::from_ratio(9_u128, 10_u128);
    STATE.save(deps.as_mut().storage, &state).unwrap();

    let err = unbond(deps.as_mut(), env, info, amount, recipient).unwrap_err();

    assert!(matches!(err, ContractError::InvalidExchangeRate {}));
}

#[test]
fn test_unbond_must_fail_if_invalid_recipient() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();

    let sender = api.addr_make("sender");

    let amount = Uint128::new(1000);
    let info = message_info(&sender, &[]);

    let recipient = Recipient::Zkgm {
        address: sender.to_string(),
        channel_id: 0,
    };

    let status = Status {
        bond_is_paused: false,
        unbond_is_paused: false,
    };
    STATUS.save(deps.as_mut().storage, &status).unwrap();

    let mut state = mock_state();
    state.exchange_rate = Decimal::one();
    STATE.save(deps.as_mut().storage, &state).unwrap();

    let err = unbond(deps.as_mut(), env, info, amount, recipient).unwrap_err();

    assert!(matches!(err, ContractError::InvalidChannelId {}));
}

#[test]
fn test_unbond_must_fail_if_missing_allowance() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();

    let sender = api.addr_make("sender");

    let amount = Uint128::new(1000);
    let info = message_info(&sender, &[]);

    let recipient = Recipient::OnChain {
        address: sender.clone(),
    };

    let status = Status {
        bond_is_paused: false,
        unbond_is_paused: false,
    };
    STATUS.save(deps.as_mut().storage, &status).unwrap();

    let mut state = mock_state();
    state.exchange_rate = Decimal::one();
    STATE.save(deps.as_mut().storage, &state).unwrap();

    let params = mock_parameters();
    PARAMETERS.save(deps.as_mut().storage, &params).unwrap();

    let allowance = amount - Uint128::one();
    deps.querier.update_wasm(move |_query| {
        let res = to_json_binary(&AllowanceResponse {
            allowance,
            ..Default::default()
        })
        .unwrap();
        cosmwasm_std::SystemResult::Ok(cosmwasm_std::ContractResult::Ok(res))
    });

    let err = unbond(deps.as_mut(), env, info, amount, recipient).unwrap_err();

    let ContractError::InsufficientAllowance {
        allowance: allowance_result,
        required,
    } = err
    else {
        panic!("wrong err, got {}", err)
    };

    assert_eq!(allowance_result, allowance);
    assert_eq!(required, amount);
}

#[test]
fn test_receive_must_fail_if_paused() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    let status = Status {
        bond_is_paused: false,
        unbond_is_paused: true,
    };
    STATUS.save(deps.as_mut().storage, &status).unwrap();

    let err = receive(
        deps.as_mut(),
        env,
        info,
        cw20::Cw20ReceiveMsg {
            sender: sender.to_string(),
            amount: Uint128::one(),
            msg: Binary::default(),
        },
    )
    .unwrap_err();

    assert!(matches!(
        err,
        ContractError::FunctionalityUnderMaintenance {}
    ))
}

#[test]
fn test_receive_must_fail_if_bad_sender() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    let status = Status {
        bond_is_paused: false,
        unbond_is_paused: false,
    };
    STATUS.save(deps.as_mut().storage, &status).unwrap();

    let params = mock_parameters();
    PARAMETERS.save(deps.as_mut().storage, &params).unwrap();

    let err = receive(
        deps.as_mut(),
        env,
        info,
        cw20::Cw20ReceiveMsg {
            sender: sender.to_string(),
            amount: Uint128::one(),
            msg: Binary::default(),
        },
    )
    .unwrap_err();

    assert!(matches!(err, ContractError::Unauthorized {}))
}

#[test]
fn test_submit_batch_must_fail_if_sender_not_owner() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    cw_ownable::initialize_owner(
        deps.as_mut().storage,
        &api,
        Some(api.addr_make("owner").as_str()),
    )
    .unwrap();

    let err = submit_batch(deps.as_mut(), env, info).unwrap_err();

    assert!(matches!(
        err,
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
    ))
}

#[test]
fn test_set_batch_received_amount_must_fail_if_sender_not_owner() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    cw_ownable::initialize_owner(
        deps.as_mut().storage,
        &api,
        Some(api.addr_make("owner").as_str()),
    )
    .unwrap();

    let err = set_batch_received_amount(deps.as_mut(), env, info, 0, Uint128::one()).unwrap_err();

    assert!(matches!(
        err,
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
    ))
}

#[test]
fn test_process_rewards_must_fail_if_sender_not_owner() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    cw_ownable::initialize_owner(
        deps.as_mut().storage,
        &api,
        Some(api.addr_make("owner").as_str()),
    )
    .unwrap();

    let err = process_rewards(deps.as_mut(), env, info).unwrap_err();

    assert!(matches!(
        err,
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
    ))
}

#[test]
fn test_update_ownership_must_fail_if_sender_not_owner() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    cw_ownable::initialize_owner(
        deps.as_mut().storage,
        &api,
        Some(api.addr_make("owner").as_str()),
    )
    .unwrap();

    let err = update_ownership(
        deps.as_mut(),
        env,
        info,
        cw_ownable::Action::AcceptOwnership {},
    )
    .unwrap_err();

    assert!(matches!(
        err,
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
    ))
}

#[test]
fn test_set_parameters_must_fail_if_sender_not_owner() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    cw_ownable::initialize_owner(
        deps.as_mut().storage,
        &api,
        Some(api.addr_make("owner").as_str()),
    )
    .unwrap();

    let err = set_parameters(
        deps.as_mut(),
        env,
        info,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap_err();

    assert!(matches!(
        err,
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
    ))
}

#[test]
fn test_process_batch_withdrawal_must_fail_if_sender_not_owner() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    cw_ownable::initialize_owner(
        deps.as_mut().storage,
        &api,
        Some(api.addr_make("owner").as_str()),
    )
    .unwrap();

    let err = process_batch_withdrawal(deps.as_mut(), env, info, 0, vec![]).unwrap_err();

    assert!(matches!(
        err,
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
    ))
}

#[test]
fn test_update_validators_must_fail_if_sender_not_owner() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    cw_ownable::initialize_owner(
        deps.as_mut().storage,
        &api,
        Some(api.addr_make("owner").as_str()),
    )
    .unwrap();

    let err = update_validators(deps.as_mut(), env, info, vec![]).unwrap_err();

    assert!(matches!(
        err,
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
    ))
}

#[test]
fn test_update_quote_token_must_fail_if_sender_not_owner() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    cw_ownable::initialize_owner(
        deps.as_mut().storage,
        &api,
        Some(api.addr_make("owner").as_str()),
    )
    .unwrap();

    let err = update_quote_token(
        deps.as_mut(),
        env,
        info,
        0,
        QuoteToken {
            channel_id: 0,
            quote_token: String::new(),
            lst_quote_token: String::new(),
        },
    )
    .unwrap_err();

    assert!(matches!(
        err,
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
    ))
}

#[test]
fn test_migrate_reward_must_fail_if_sender_not_owner() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    cw_ownable::initialize_owner(
        deps.as_mut().storage,
        &api,
        Some(api.addr_make("owner").as_str()),
    )
    .unwrap();

    let err = migrate_reward(deps.as_mut(), env, info, 0).unwrap_err();

    assert!(matches!(
        err,
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
    ))
}

#[test]
fn test_set_status_must_fail_if_sender_not_owner() {
    let mut deps = mock_dependencies();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    cw_ownable::initialize_owner(
        deps.as_mut().storage,
        &api,
        Some(api.addr_make("owner").as_str()),
    )
    .unwrap();

    let err = set_status(
        deps.as_mut(),
        info,
        Status {
            bond_is_paused: false,
            unbond_is_paused: false,
        },
    )
    .unwrap_err();

    assert!(matches!(
        err,
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
    ))
}

#[test]
fn test_set_or_remove_chain_must_fail_if_sender_not_owner() {
    let mut deps = mock_dependencies();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    cw_ownable::initialize_owner(
        deps.as_mut().storage,
        &api,
        Some(api.addr_make("owner").as_str()),
    )
    .unwrap();

    let err = set_bond_chain(
        deps.as_mut(),
        info.clone(),
        crate::state::ZkgmChain {
            name: String::new(),
            chain_id: String::new(),
            ucs03_channel_id: 0,
            prefix: String::new(),
        },
    )
    .unwrap_err();

    assert!(matches!(
        err,
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
    ));

    let err = remove_bond_chain(deps.as_mut(), info, 0).unwrap_err();

    assert!(matches!(
        err,
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
    ));
}

#[test]
fn test_set_bond_chain() {
    let mut deps = mock_dependencies();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    cw_ownable::initialize_owner(deps.as_mut().storage, &api, Some(sender.as_str())).unwrap();

    set_bond_chain(
        deps.as_mut(),
        info.clone(),
        crate::state::ZkgmChain {
            name: String::new(),
            chain_id: String::new(),
            ucs03_channel_id: 3,
            prefix: "bbn".into(),
        },
    )
    .unwrap();
    let chains = query_bond_chains(&deps.storage).unwrap();

    assert_eq!(chains.len(), 1);
    assert_eq!(chains.first().unwrap().ucs03_channel_id, 3);
}

#[test]
fn test_set_unbond_chain() {
    let mut deps = mock_dependencies();
    let api = deps.api.clone();
    let sender = api.addr_make("sender");
    let info = message_info(&sender, &[]);

    cw_ownable::initialize_owner(deps.as_mut().storage, &api, Some(sender.as_str())).unwrap();

    set_unbond_chain(
        deps.as_mut(),
        info.clone(),
        crate::state::ZkgmChain {
            name: String::new(),
            chain_id: String::new(),
            ucs03_channel_id: 2,
            prefix: "bbn".into(),
        },
    )
    .unwrap();
    let chains = query_unbond_chains(&deps.storage).unwrap();

    assert_eq!(chains.len(), 1);
    assert_eq!(chains.first().unwrap().ucs03_channel_id, 2);
}
