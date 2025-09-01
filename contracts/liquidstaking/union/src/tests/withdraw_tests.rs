use cosmwasm_std::{
    Addr, Coin, CosmosMsg, StdError,
    testing::{message_info, mock_env},
};
use depolama::StorageExt;

use crate::{
    contract::execute,
    msg::ExecuteMsg,
    state::{AccountingStateStore, Batches, UnstakeRequests},
    tests::test_helper::{NATIVE_TOKEN, UNION1, UNION2, init},
    types::{Batch, BatchId, Staker, UnstakeRequest, UnstakeRequestKey},
};

#[test]
fn withdraw() {
    let mut deps = init();

    let _ = deps
        .storage
        .upsert_item::<AccountingStateStore, StdError>(|s| {
            let mut s = s.unwrap();
            s.total_bonded_native_tokens = 300_000;
            s.total_issued_lst = 130_000;
            Ok(s)
        })
        .unwrap();

    let batch_id = BatchId::from_raw(1).unwrap();

    let staker_1 = Staker::Local {
        address: UNION1.to_string(),
    };

    deps.storage.write::<UnstakeRequests>(
        &UnstakeRequestKey {
            batch_id,
            staker_hash: staker_1.hash(),
        },
        &UnstakeRequest {
            batch_id,
            staker: staker_1,
            amount: 40_000,
        },
    );

    let staker_2 = Staker::Local {
        address: UNION2.to_string(),
    };

    deps.storage.write::<UnstakeRequests>(
        &UnstakeRequestKey {
            batch_id,
            staker_hash: staker_2.hash(),
        },
        &UnstakeRequest {
            batch_id,
            staker: staker_2,
            amount: 90_000,
        },
    );

    let batch = Batch {
        total_lst_to_burn: 130_000,
        unstake_requests_count: 2,
        state: crate::types::BatchState::Received {
            received_native_unstaked: 130_000,
        },
    };

    deps.storage.write::<Batches>(&batch_id, &batch);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&Addr::unchecked(UNION1), &vec![]),
        ExecuteMsg::Withdraw {
            staker: Addr::unchecked(UNION1),
            withdraw_to_address: Addr::unchecked(UNION1),
            batch_id,
        },
    )
    .unwrap();

    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
            to_address: UNION1.to_string(),
            amount: vec![Coin {
                denom: NATIVE_TOKEN.to_string(),
                amount: 40_000u128.into()
            }]
        })
    );

    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&Addr::unchecked(UNION2), &vec![]),
        ExecuteMsg::Withdraw {
            staker: Addr::unchecked(UNION2),
            withdraw_to_address: Addr::unchecked(UNION2),
            batch_id,
        },
    )
    .unwrap();

    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
            to_address: UNION2.to_string(),
            amount: vec![Coin {
                denom: NATIVE_TOKEN.to_string(),
                amount: 90_000u128.into()
            }]
        })
    );
}

// #[test]
// fn withdraw_slashing() {
//     let mut deps = init();
//     let env = mock_env();
//     let mut state = STATE.load(&deps.storage).unwrap();

//     state.total_liquid_stake_token = Uint128::from(130_000u128);
//     state.total_bonded_native_tokens = Uint128::from(300_000u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     let mut pending_batch: Batch =
//         Batch::new_pending(1, Uint128::new(130_000), env.block.time.seconds() + 10_000);
//     new_unstake_request(
//         &mut deps.as_mut(),
//         "bob".to_string(),
//         1,
//         Uint128::from(40_000u128),
//     )
//     .unwrap();
//     new_unstake_request(
//         &mut deps.as_mut(),
//         "tom".to_string(),
//         1,
//         Uint128::from(90_000u128),
//     )
//     .unwrap();
//     let res = BATCHES.save(&mut deps.storage, 1, &pending_batch);
//     assert!(res.is_ok());

//     // batch ready
//     pending_batch.received_native_unstaked = Some(Uint128::new(990_000)); // slashing happened
//     pending_batch.state = crate::types::BatchState::Received;
//     let res = BATCHES.save(&mut deps.storage, 1, &pending_batch);
//     assert!(res.is_ok());

//     // success
//     let msg = ExecuteMsg::Withdraw {
//         batch_id: pending_batch.id,
//     };
//     let info = mock_info("bob", &[]);
//     let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
//     assert!(res.is_ok());
//     let messages = res.unwrap().messages;
//     assert_eq!(messages.len(), 2); // withdraw and redemption rate update

//     let msg = QueryMsg::UnstakeRequests {
//         user: Addr::unchecked("bob"),
//     };
//     let res = query(deps.as_ref(), env.clone(), msg);
//     assert!(res.is_ok());
//     let resp: Vec<UnstakeRequest> = from_json(res.unwrap()).unwrap();

//     assert!(resp.is_empty());

//     let config = CONFIG.load(&deps.storage).unwrap();
//     let coin = Coin {
//         denom: config.protocol_chain_config.native_token_denom.clone(),
//         amount: "304615".to_string(), //304615.384... = 304615
//     };

//     // check the MsgSend
//     let coins = vec![coin];
//     assert_eq!(
//         messages[0],
//         SubMsg {
//             id: 0,
//             msg: <MsgSend as Into<CosmosMsg>>::into(MsgSend {
//                 from_address: Addr::unchecked(MOCK_CONTRACT_ADDR).to_string(),
//                 to_address: "bob".to_string(),
//                 amount: coins,
//             }),
//             gas_limit: None,
//             reply_on: ReplyOn::Never,
//         }
//     );

//     // Tom withdraw
//     let msg = ExecuteMsg::Withdraw {
//         batch_id: pending_batch.id,
//     };
//     let info = mock_info("tom", &[]);
//     let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
//     assert!(res.is_ok());
//     let messages = res.unwrap().messages;
//     assert_eq!(messages.len(), 2); // withdraw and redemption/purchase rate update

//     let msg = QueryMsg::UnstakeRequests {
//         user: Addr::unchecked("tom"),
//     };
//     let res = query(deps.as_ref(), env.clone(), msg);
//     assert!(res.is_ok());
//     let resp: Vec<UnstakeRequest> = from_json(res.unwrap()).unwrap();

//     assert!(resp.is_empty());

//     let config = CONFIG.load(&deps.storage).unwrap();
//     let coin = Coin {
//         denom: config.protocol_chain_config.native_token_denom.clone(),
//         amount: "685384".to_string(), //685,384.615... = 685384
//     };

//     // check the MsgSend
//     let coins = vec![coin];
//     assert_eq!(
//         messages[0],
//         SubMsg {
//             id: 0,
//             msg: <MsgSend as Into<CosmosMsg>>::into(MsgSend {
//                 from_address: Addr::unchecked(MOCK_CONTRACT_ADDR).to_string(),
//                 to_address: "tom".to_string(),
//                 amount: coins,
//             }),
//             gas_limit: None,
//             reply_on: ReplyOn::Never,
//         }
//     );
// }

// #[test]
// fn fee_withdraw() {
//     let mut deps = init();
//     let env = mock_env();
//     let mut state = STATE.load(&deps.storage).unwrap();
//     state.total_fees = Uint128::from(1000u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     let msg = ExecuteMsg::FeeWithdraw {
//         amount: Uint128::from(2000u128),
//     };
//     let info = mock_info("bob", &[]);
//     let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
//     assert!(res.is_err()); // because not admin

//     let info = mock_info(ADMIN, &[]);
//     let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
//     assert!(res.is_err()); // because too high amount

//     let msg = ExecuteMsg::FeeWithdraw {
//         amount: Uint128::from(1000u128),
//     };
//     let info = mock_info(ADMIN, &[]);
//     let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
//     assert!(res.is_ok());
//     assert_eq!(
//         res.unwrap().messages[0],
//         SubMsg {
//             id: 0,
//             msg: <MsgSend as Into<CosmosMsg>>::into(MsgSend {
//                 from_address: Addr::unchecked(MOCK_CONTRACT_ADDR).to_string(),
//                 to_address: OSMO1.to_string(),
//                 amount: vec![Coin {
//                     denom: NATIVE_TOKEN.to_string(),
//                     amount: "1000".to_string()
//                 }],
//             }),
//             gas_limit: None,
//             reply_on: ReplyOn::Never,
//         }
//     );
// }
