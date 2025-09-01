use cosmwasm_std::{
    attr,
    testing::{message_info, mock_env},
    to_json_binary, Addr, Coin, CosmosMsg, Event, StdError, Timestamp, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use depolama::StorageExt;

use super::test_helper::{NATIVE_TOKEN, UNION3};
use crate::{
    contract::execute,
    msg::ExecuteMsg,
    state::{AccountingStateStore, Batches, UnstakeRequests},
    tests::test_helper::{init, LIQUID_STAKE_TOKEN_ADDRESS, UNION1, UNION2},
    types::{BatchId, BatchState, Staker, UnstakeRequest, UnstakeRequestKey},
};

#[test]
fn unbond_works() {
    let mut deps = init();

    deps.storage
        .upsert_item::<AccountingStateStore, StdError>(|s| {
            let mut s = s.unwrap();
            s.total_bonded_native_tokens = 1_100;
            s.total_issued_lst = 1_000;
            Ok(s)
        })
        .unwrap();

    let info = message_info(&Addr::unchecked(UNION1), &[]);
    let union1_amount_1 = 1_000u128.into();
    let res = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::Unbond {
            staker: Addr::unchecked(UNION1),
            amount: union1_amount_1,
        },
    )
    .unwrap();

    let staker = Staker::Local {
        address: UNION1.to_string(),
    };

    let batch_id = BatchId::from_raw(1).unwrap();

    let unstake_req = deps
        .storage
        .read::<UnstakeRequests>(&UnstakeRequestKey {
            batch_id,
            staker_hash: staker.hash(),
        })
        .unwrap();

    // a new unstake request is created
    assert_eq!(
        unstake_req,
        UnstakeRequest {
            batch_id,
            staker: staker.clone(),
            amount: union1_amount_1.u128()
        }
    );

    let batch = deps.storage.read::<Batches>(&batch_id).unwrap();

    // batch is adjusted accordingly
    assert_eq!(batch.total_lst_to_burn, union1_amount_1.u128());
    assert_eq!(batch.unstake_requests_count, 1);

    // lst token is locked in the lst contract
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: LIQUID_STAKE_TOKEN_ADDRESS.into(),
            msg: to_json_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: UNION1.into(),
                recipient: mock_env().contract.address.to_string(),
                amount: union1_amount_1
            })
            .unwrap(),
            funds: vec![]
        })
    );

    // we expect no further messages
    assert_eq!(res.messages.len(), 1);

    assert_eq!(
        res.events[0],
        Event::new("local_unbond")
            .add_attribute("staker_hash", staker.hash().to_string())
            .add_attribute("batch", "1")
            .add_attribute("amount", union1_amount_1)
            .add_attribute("is_new_request", "true"),
    );

    assert_eq!(
        res.attributes,
        vec![attr("staker_type", "local"), attr("staker_address", UNION1),]
    );

    let union1_amount_2 = 3_500u128.into();
    let res = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::Unbond {
            staker: Addr::unchecked(UNION1),
            amount: union1_amount_2,
        },
    )
    .unwrap();

    // is_new_request is now false
    assert_eq!(
        res.events[0],
        Event::new("local_unbond")
            .add_attribute("staker_hash", staker.hash().to_string())
            .add_attribute("batch", "1")
            .add_attribute("amount", union1_amount_2)
            .add_attribute("is_new_request", "false"),
    );

    let unstake_req = deps
        .storage
        .read::<UnstakeRequests>(&UnstakeRequestKey {
            batch_id,
            staker_hash: staker.hash(),
        })
        .unwrap();

    // the unstake request is updated
    assert_eq!(
        unstake_req,
        UnstakeRequest {
            batch_id,
            staker: staker.clone(),
            amount: (union1_amount_1 + union1_amount_2).u128()
        }
    );

    let batch = deps.storage.read::<Batches>(&batch_id).unwrap();

    assert_eq!(
        batch.total_lst_to_burn,
        (union1_amount_1 + union1_amount_2).u128()
    );
    // unstake requests count is gonna stay the same since im updating my request
    assert_eq!(batch.unstake_requests_count, 1);

    let union2_amount_1 = 4528u128.into();

    // a new unstake request
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&Addr::unchecked(UNION2), &[]),
        ExecuteMsg::Unbond {
            staker: Addr::unchecked(UNION2),
            amount: union2_amount_1,
        },
    )
    .unwrap();

    let staker = Staker::Local {
        address: UNION2.to_string(),
    };

    let unstake_req = deps
        .storage
        .read::<UnstakeRequests>(&UnstakeRequestKey {
            batch_id,
            staker_hash: staker.hash(),
        })
        .unwrap();

    // the unstake request is updated
    assert_eq!(
        unstake_req,
        UnstakeRequest {
            batch_id,
            staker,
            amount: union2_amount_1.u128()
        }
    );

    let batch = deps.storage.read::<Batches>(&batch_id).unwrap();

    assert_eq!(
        batch.total_lst_to_burn,
        (union1_amount_1 + union1_amount_2 + union2_amount_1).u128()
    );
    // this time the unstake request count is incremented since a new user unstaked
    assert_eq!(batch.unstake_requests_count, 2);
}

#[test]
fn receive_unstaked_tokens_works() {
    let mut deps = init();

    deps.storage
        .upsert_item::<AccountingStateStore, StdError>(|s| {
            let mut s = s.unwrap();
            s.total_bonded_native_tokens = 5_000;
            s.total_issued_lst = 5_000;
            Ok(s)
        })
        .unwrap();

    // UNION1 unbonds 1532 tokens
    let union1_unbond_amount = 1532u128.into();
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&Addr::unchecked(UNION1), &[]),
        ExecuteMsg::Unbond {
            staker: Addr::unchecked(UNION1),
            amount: union1_unbond_amount,
        },
    )
    .unwrap();

    // UNION2 unbonds 1200 tokens
    let union2_unbond_amount = 1200u128.into();
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&Addr::unchecked(UNION2), &[]),
        ExecuteMsg::Unbond {
            staker: Addr::unchecked(UNION2),
            amount: union2_unbond_amount,
        },
    )
    .unwrap();

    // UNION3 unbonds 500 tokens
    let union3_unbond_amount = 500u128.into();
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&Addr::unchecked(UNION3), &[]),
        ExecuteMsg::Unbond {
            staker: Addr::unchecked(UNION3),
            amount: union3_unbond_amount,
        },
    )
    .unwrap();

    let batch_id = BatchId::from_raw(1).unwrap();

    let mut env = mock_env();
    env.block.time = if let BatchState::Pending { submit_time } =
        deps.storage.read::<Batches>(&batch_id).unwrap().state
    {
        Timestamp::from_seconds(submit_time + 1)
    } else {
        panic!("invalid state")
    };

    // batch is submitted so that we can receive the unstaked tokens
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&Addr::unchecked(UNION1), &[]),
        ExecuteMsg::SubmitBatch {},
    )
    .unwrap();

    let mut env = mock_env();
    env.block.time = if let BatchState::Submitted { receive_time, .. } =
        deps.storage.read::<Batches>(&batch_id).unwrap().state
    {
        Timestamp::from_seconds(receive_time + 1)
    } else {
        panic!("invalid state")
    };

    let total_unbond_amount = union1_unbond_amount + union2_unbond_amount + union3_unbond_amount;
    let res = execute(
        deps.as_mut(),
        env,
        message_info(
            &Addr::unchecked(UNION1),
            &[Coin {
                denom: NATIVE_TOKEN.into(),
                amount: total_unbond_amount,
            }],
        ),
        ExecuteMsg::ReceiveUnstakedTokens { batch_id },
    )
    .unwrap();

    // the batch state is updated with the correct unbond amount
    let batch = deps.storage.read::<Batches>(&batch_id).unwrap();
    assert_eq!(
        batch.state,
        BatchState::Received {
            received_native_unstaked: total_unbond_amount.u128()
        }
    );

    // the event is emitted correctly
    assert_eq!(
        res.events[0],
        Event::new("receive_unstaked_tokens")
            .add_attribute("batch", batch_id.to_string())
            .add_attribute("amount", total_unbond_amount.to_string())
    );
}

// #[test]
// fn invalid_denom_liquid_unstake() {
//     let mut deps = init();

//     let mut state = STATE.load(&deps.storage).unwrap();

//     state.total_liquid_stake_token = Uint128::from(100_000u128);
//     state.total_bonded_native_tokens = Uint128::from(300_000u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     let info = mock_info("bob", &coins(1000, "factory/bob/stTIA"));
//     let msg = ExecuteMsg::Unbond {};

//     let res = execute(deps.as_mut(), mock_env(), info, msg);

//     assert!(res.is_err());
// }

// #[test]
// fn receive_unstaked_tokens() {
//     let mut deps = init();
//     let env = mock_env();

//     let mut state = STATE.load(&deps.storage).unwrap();
//     let config = CONFIG.load(&deps.storage).unwrap();

//     state.total_liquid_stake_token = Uint128::from(100_000u128);
//     state.total_bonded_native_tokens = Uint128::from(300_000u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     let msg = ExecuteMsg::ReceiveUnstakedTokens { batch_id: 1 };

//     let sender = derive_intermediate_sender(
//         &config.protocol_chain_config.ibc_channel_id,
//         config.native_chain_config.staker_address.as_str(),
//         config.protocol_chain_config.account_address_prefix.as_str(),
//     )
//     .unwrap();

//     let info = mock_info(
//         &sender,
//         &[cosmwasm_std::Coin {
//             amount: Uint128::from(100u128),
//             denom: config.protocol_chain_config.native_token_denom.clone(),
//         }],
//     );

//     let mut batch: Batch = BATCHES.load(&deps.storage, 1).unwrap();
//     batch.expected_native_unstaked = Some(Uint128::new(100));
//     batch.update_status(BatchState::Pending, Some(env.block.time.seconds() - 1));
//     BATCHES.save(&mut deps.storage, 1, &batch).unwrap();

//     let res: Result<cosmwasm_std::Response, crate::error::ContractError> =
//         execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
//     assert!(res.is_err()); // batch not submitted

//     batch.update_status(BatchState::Submitted, Some(env.block.time.seconds() + 1));
//     BATCHES.save(&mut deps.storage, 1, &batch).unwrap();

//     let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
//     assert!(res.is_err()); // batch not ready

//     batch.update_status(BatchState::Submitted, Some(env.block.time.seconds() - 1));
//     BATCHES.save(&mut deps.storage, 1, &batch).unwrap();

//     execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
// }

// #[test]
// fn invalid_amount_liquid_unstake() {
//     let mut deps = init();

//     let mut state = STATE.load(&deps.storage).unwrap();

//     state.total_liquid_stake_token = Uint128::from(100_000u128);
//     state.total_bonded_native_tokens = Uint128::from(300_000u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     let info = mock_info(
//         "bob",
//         &coins(
//             1_000_000_000,
//             format!("factory/cosmos2contract/{}", LIQUID_STAKE_TOKEN_DENOM),
//         ),
//     );
//     let msg = ExecuteMsg::Unbond {};

//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
//     let resp = res.unwrap();

//     let attrs = resp.attributes;
//     assert_eq!(attrs[0].value, "liquid_unstake");
//     assert_eq!(attrs[1].value, "bob"); // sender
//     assert_eq!(attrs[2].value, "1"); // batch id
//     assert_eq!(attrs[3].value, "1000000000");

//     // total_liquid_stake_token = 100_000
//     // unstake = 1_000_000_000
//     let batch = BATCHES.load(&deps.storage, 1).unwrap();
//     assert_eq!(
//         batch.batch_total_liquid_stake,
//         Uint128::from(1_000_000_000u128)
//     );

//     // Submit batch
//     // currently disabled auto batch submit
//     // assert_eq!(resp.messages.len(), 1);
//     let mut env = mock_env();
//     let config = CONFIG.load(&deps.storage).unwrap();

//     env.block.time = env.block.time.plus_seconds(config.batch_period + 1);
//     let msg = ExecuteMsg::SubmitBatch {};
//     let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
//     assert!(res.is_err());

//     // check the state
//     state = STATE.load(&deps.storage).unwrap();
//     assert_eq!(state.total_liquid_stake_token, Uint128::from(100000u128));
//     assert_eq!(state.total_bonded_native_tokens, Uint128::from(300000u128));

//     // check the batch
//     let batch = BATCHES.load(&deps.storage, 1).unwrap();
//     assert_eq!(
//         batch.batch_total_liquid_stake,
//         Uint128::from(1000000000u128)
//     );
//     assert_eq!(batch.state, BatchState::Pending);
// }

// #[test]
// fn total_liquid_stake_token_with_zero() {
//     let mut deps = init();

//     let mut state = STATE.load(&deps.storage).unwrap();

//     state.total_liquid_stake_token = Uint128::from(0u128);
//     state.total_bonded_native_tokens = Uint128::from(300_000u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     let info = mock_info(
//         "bob",
//         &coins(
//             1_000_000_000,
//             format!("factory/cosmos2contract/{}", LIQUID_STAKE_TOKEN_DENOM),
//         ),
//     );
//     let msg = ExecuteMsg::Unbond {};

//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
//     let resp = res.unwrap();

//     let attrs = resp.attributes;
//     assert_eq!(attrs[0].value, "liquid_unstake");
//     assert_eq!(attrs[1].value, "bob"); // sender
//     assert_eq!(attrs[2].value, "1"); // batch id
//     assert_eq!(attrs[3].value, "1000000000");

//     // total_liquid_stake_token = 100_000
//     // unstake = 1_000_000_000
//     let batch = BATCHES.load(&deps.storage, 1).unwrap();
//     assert_eq!(
//         batch.batch_total_liquid_stake,
//         Uint128::from(1_000_000_000u128)
//     );

//     // Submit batch
//     // currently disabled auto batch submit
//     // assert_eq!(resp.messages.len(), 1);
//     let mut env = mock_env();
//     let config = CONFIG.load(&deps.storage).unwrap();

//     env.block.time = env.block.time.plus_seconds(config.batch_period + 1);
//     let msg = ExecuteMsg::SubmitBatch {};
//     let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
//     assert!(res.is_err());

//     // check the state
//     state = STATE.load(&deps.storage).unwrap();
//     assert_eq!(state.total_liquid_stake_token, Uint128::from(0u128));
//     assert_eq!(state.total_bonded_native_tokens, Uint128::from(300000u128));

//     // check the batch
//     let batch = BATCHES.load(&deps.storage, 1).unwrap();
//     assert_eq!(
//         batch.batch_total_liquid_stake,
//         Uint128::from(1000000000u128)
//     );
//     assert_eq!(batch.state, BatchState::Pending);
// }

// #[test]
// fn claimable_batches() {
//     let mut deps = init();

//     let mut state = STATE.load(&deps.storage).unwrap();

//     state.total_liquid_stake_token = Uint128::from(100_000u128);
//     state.total_bonded_native_tokens = Uint128::from(300_000u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     let mut batch_1 = Batch::new_pending(1, Uint128::from(1000u128), 1000);
//     batch_1.expected_native_unstaked = Some(Uint128::new(1000));
//     new_unstake_request(
//         &mut deps.as_mut(),
//         "bob".to_string(),
//         1,
//         Uint128::from(1000u128),
//     )
//     .unwrap();
//     let mut batch_2 = Batch::new_pending(2, Uint128::from(1000u128), 1000);
//     batch_2.expected_native_unstaked = Some(Uint128::new(1000));
//     new_unstake_request(
//         &mut deps.as_mut(),
//         "bob".to_string(),
//         2,
//         Uint128::from(1000u128),
//     )
//     .unwrap();
//     let res = BATCHES.save(&mut deps.storage, 1, &batch_1);
//     assert!(res.is_ok());
//     let res = BATCHES.save(&mut deps.storage, 2, &batch_2);
//     assert!(res.is_ok());

//     let unstake_requests_res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::UnstakeRequests {
//             user: Addr::unchecked("bob"),
//         },
//     );
//     assert!(unstake_requests_res.is_ok());
//     let unstake_requests_res = from_json::<Vec<UnstakeRequest>>(&unstake_requests_res.unwrap());
//     assert!(unstake_requests_res.is_ok());
//     let unstake_requests = unstake_requests_res.unwrap();
//     assert_eq!(unstake_requests.len(), 2);

//     // receive tokens for batch 1
//     let mut batch: Batch = BATCHES.load(&deps.storage, 1).unwrap();
//     batch.update_status(BatchState::Submitted, Some(1000));
//     let res = BATCHES.save(&mut deps.storage, 1, &batch);
//     assert!(res.is_ok());

//     let msg = ExecuteMsg::ReceiveUnstakedTokens { batch_id: 1 };
//     let config = CONFIG.load(&deps.storage).unwrap();

//     let sender = derive_intermediate_sender(
//         &config.protocol_chain_config.ibc_channel_id,
//         config.native_chain_config.staker_address.as_str(),
//         config.protocol_chain_config.account_address_prefix.as_str(),
//     )
//     .unwrap();

//     let info = mock_info(
//         &sender,
//         &[cosmwasm_std::Coin {
//             amount: Uint128::from(1000u128),
//             denom: config.protocol_chain_config.native_token_denom,
//         }],
//     );
//     execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     let unstake_requests_res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::UnstakeRequests {
//             user: Addr::unchecked("bob"),
//         },
//     );
//     assert!(unstake_requests_res.is_ok());
//     let unstake_requests_res = from_json::<Vec<UnstakeRequest>>(&unstake_requests_res.unwrap());
//     assert!(unstake_requests_res.is_ok());
//     let unstake_requests = unstake_requests_res.unwrap();
//     assert_eq!(unstake_requests.first().unwrap().batch_id, 1);
// }
