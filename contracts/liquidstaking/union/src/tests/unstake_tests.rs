use cosmwasm_std::{
    attr,
    testing::{message_info, mock_env},
    to_json_binary, Addr, Attribute, Coin, CosmosMsg, Timestamp, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

use super::test_helper::{NATIVE_TOKEN, UNION3};
use crate::{
    contract::execute,
    msg::ExecuteMsg,
    state::{unstake_requests, UnstakeRequest, BATCHES, STATE},
    tests::test_helper::{init, LIQUID_STAKE_TOKEN_ADDRESS, UNION1, UNION2},
    types::BatchState,
};

#[test]
fn unbond_works() {
    let mut deps = init();

    let mut state = STATE.load(&deps.storage).unwrap();

    state.total_bonded_native_tokens = Uint128::from(1_100u128);
    state.total_issued_lst = Uint128::from(1_000u128);
    STATE.save(&mut deps.storage, &state).unwrap();

    let info = message_info(&Addr::unchecked(UNION1), &[]);
    let union1_amount_1 = 1_000u128.into();
    let res = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::Unbond {
            staker: UNION1.as_bytes().into(),
            amount: union1_amount_1,
        },
    )
    .unwrap();

    let unstake_req = unstake_requests()
        .load(&deps.storage, (1, UNION1.as_bytes().into()))
        .unwrap();

    // a new unstake request is created
    assert_eq!(
        unstake_req,
        UnstakeRequest {
            batch_id: 1,
            user: UNION1.as_bytes().into(),
            amount: union1_amount_1
        }
    );

    let batch = BATCHES.load(&deps.storage, 1).unwrap();

    // batch is adjusted accordingly
    assert_eq!(batch.total_lst_to_burn, union1_amount_1);
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
        res.attributes,
        vec![
            attr("action", "unbond"),
            attr("sender", UNION1),
            attr("batch", "1"),
            attr("amount", union1_amount_1),
            attr("is_new_request", "true"),
        ]
    );

    let union1_amount_2 = 3_500u128.into();
    let _ = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::Unbond {
            staker: UNION1.as_bytes().into(),
            amount: union1_amount_2,
        },
    )
    .unwrap();

    let unstake_req = unstake_requests()
        .load(&deps.storage, (1, UNION1.as_bytes().into()))
        .unwrap();

    // the unstake request is updated
    assert_eq!(
        unstake_req,
        UnstakeRequest {
            batch_id: 1,
            user: UNION1.as_bytes().into(),
            amount: union1_amount_1 + union1_amount_2
        }
    );

    let batch = BATCHES.load(&deps.storage, 1).unwrap();

    assert_eq!(batch.total_lst_to_burn, union1_amount_1 + union1_amount_2);
    // unstake requests count is gonna stay the same since im updating my request
    assert_eq!(batch.unstake_requests_count, 1);

    let union2_amount_1 = 4528u128.into();

    // a new unstake request
    let _ = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&Addr::unchecked(UNION2), &[]),
        ExecuteMsg::Unbond {
            staker: UNION2.as_bytes().into(),
            amount: union2_amount_1,
        },
    )
    .unwrap();

    let unstake_req = unstake_requests()
        .load(&deps.storage, (1, UNION2.into()))
        .unwrap();

    // the unstake request is updated
    assert_eq!(
        unstake_req,
        UnstakeRequest {
            batch_id: 1,
            user: UNION2.as_bytes().into(),
            amount: union2_amount_1
        }
    );

    let batch = BATCHES.load(&deps.storage, 1).unwrap();

    assert_eq!(
        batch.total_lst_to_burn,
        union1_amount_1 + union1_amount_2 + union2_amount_1
    );
    // this time the unstake request count is incremented since a new user unstaked
    assert_eq!(batch.unstake_requests_count, 2);
}

#[test]
fn receive_unstaked_tokens_works() {
    let mut deps = init();

    let mut state = STATE.load(&deps.storage).unwrap();

    state.total_bonded_native_tokens = Uint128::from(5_000u128);
    state.total_issued_lst = Uint128::from(5_000u128);

    STATE.save(&mut deps.storage, &state).unwrap();

    // UNION1 unbonds 1532 tokens
    let union1_unbond_amount = 1532u128.into();
    let _ = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&Addr::unchecked(UNION1), &[]),
        ExecuteMsg::Unbond {
            staker: UNION1.as_bytes().into(),
            amount: union1_unbond_amount,
        },
    )
    .unwrap();

    // UNION2 unbonds 1200 tokens
    let union2_unbond_amount = 1200u128.into();
    let _ = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&Addr::unchecked(UNION2), &[]),
        ExecuteMsg::Unbond {
            staker: UNION2.as_bytes().into(),
            amount: union2_unbond_amount,
        },
    )
    .unwrap();

    // UNION3 unbonds 500 tokens
    let union3_unbond_amount = 500u128.into();
    let _ = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&Addr::unchecked(UNION3), &[]),
        ExecuteMsg::Unbond {
            staker: UNION3.as_bytes().into(),
            amount: union3_unbond_amount,
        },
    )
    .unwrap();

    let mut env = mock_env();
    env.block.time = if let BatchState::Pending { submit_time } =
        BATCHES.load(&deps.storage, 1).unwrap().state
    {
        Timestamp::from_seconds(submit_time + 1)
    } else {
        panic!("invalid state")
    };

    // batch is submitted so that we can receive the unstaked tokens
    let _ = execute(
        deps.as_mut(),
        env.clone(),
        message_info(&Addr::unchecked(UNION1), &[]),
        ExecuteMsg::SubmitBatch {},
    )
    .unwrap();

    let mut env = mock_env();
    env.block.time = if let BatchState::Submitted { receive_time, .. } =
        BATCHES.load(&deps.storage, 1).unwrap().state
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
        ExecuteMsg::ReceiveUnstakedTokens { batch_id: 1 },
    )
    .unwrap();

    // the batch state is updated with the correct unbond amount
    let batch = BATCHES.load(&deps.storage, 1).unwrap();
    assert_eq!(
        batch.state,
        BatchState::Received {
            received_native_unstaked: total_unbond_amount
        }
    );

    // the event is emitted correctly
    assert_eq!(
        res.attributes,
        vec![
            Attribute::new("action", "receive_unstaked_tokens"),
            Attribute::new("batch", "1"),
            Attribute::new("amount", total_unbond_amount.to_string()),
        ]
    );
}

// #[test]
// fn double_liquid_unstake() {
//     let mut deps = init();

//     let mut state = STATE.load(&deps.storage).unwrap();

//     state.total_bonded_native_tokens = Uint128::from(10_000u128);
//     state.total_liquid_stake_token = Uint128::from(100_000u128);
//     STATE.save(&mut deps.storage, &state).unwrap();
//     let msg = ExecuteMsg::Unbond {};

//     // Bob unstakes 500
//     let info = mock_info(
//         "bob",
//         &coins(
//             500,
//             format!("factory/cosmos2contract/{}", LIQUID_STAKE_TOKEN_DENOM),
//         ),
//     );
//     let mut res = execute(deps.as_mut(), mock_env(), info, msg.clone());
//     assert!(res.is_ok());

//     // Bob unstakes 1_000
//     let info = mock_info(
//         "bob",
//         &coins(
//             1_000,
//             format!("factory/cosmos2contract/{}", LIQUID_STAKE_TOKEN_DENOM),
//         ),
//     );
//     res = execute(deps.as_mut(), mock_env(), info, msg.clone());
//     assert!(res.is_ok());

//     // Check pending batch
//     let unstake_requests_records = unstake_requests()
//         .prefix(1u64)
//         .range(&deps.storage, None, None, cosmwasm_std::Order::Ascending)
//         .map(|v| v.unwrap())
//         .collect::<Vec<_>>();
//     assert!(unstake_requests_records.len() == 1);
//     assert_eq!(
//         unstake_requests_records.first().unwrap().1.amount,
//         Uint128::from(1500u128)
//     );

//     // Alice unstakes 5_000
//     let info = mock_info(
//         "alice",
//         &coins(
//             5_000,
//             format!("factory/cosmos2contract/{}", LIQUID_STAKE_TOKEN_DENOM),
//         ),
//     );
//     res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
//     assert!(res.is_ok());

//     // Check pending batch
//     let pending_batch = BATCHES.load(&deps.storage, 1).unwrap();
//     assert_eq!(
//         pending_batch.batch_total_liquid_stake,
//         Uint128::from(6_500u128)
//     );

//     // submit batch
//     let mut env = mock_env();
//     let config = CONFIG.load(&deps.storage).unwrap();
//     env.block.time = env.block.time.plus_seconds(config.batch_period + 1);

//     let msg = ExecuteMsg::SubmitBatch {};
//     let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
//     let resp = res.unwrap();
//     let attrs = resp.attributes;
//     assert_eq!(attrs[0].value, "submit_batch");
//     assert_eq!(attrs[1].value, "1");
//     assert_eq!(attrs[2].value, "6500");
//     assert_eq!(attrs[3].value, "650");

//     let messages = resp.messages;
//     assert_eq!(messages.len(), 2); // batch submit and redemption/purchase rate update
//     assert_eq!(
//         messages[0],
//         SubMsg {
//             id: 0,
//             msg: <MsgBurn as Into<CosmosMsg>>::into(MsgBurn {
//                 sender: Addr::unchecked(MOCK_CONTRACT_ADDR).to_string(),
//                 amount: Some(Coin {
//                     denom: format!("factory/cosmos2contract/{}", LIQUID_STAKE_TOKEN_DENOM),
//                     amount: "6500".to_string(),
//                 }),
//                 burn_from_address: Addr::unchecked(MOCK_CONTRACT_ADDR).to_string(),
//             }),
//             gas_limit: None,
//             reply_on: ReplyOn::Never,
//         }
//     );

//     // check the batch
//     let batch = BATCHES.load(&deps.storage, 1).unwrap();
//     assert_eq!(batch.batch_total_liquid_stake, Uint128::from(6500u128));
//     assert_eq!(batch.state, BatchState::Submitted);
// }

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
