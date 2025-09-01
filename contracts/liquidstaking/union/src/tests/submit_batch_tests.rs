use cosmwasm_std::{
    Addr, Coin, CosmosMsg, Event, StdError, Timestamp, WasmMsg,
    testing::{message_info, mock_env},
    to_json_binary,
};
use cw20::Cw20ExecuteMsg;
use depolama::StorageExt;

use crate::{
    contract::execute,
    msg::ExecuteMsg,
    state::{AccountingStateStore, Batches, ConfigStore, PendingBatchId},
    tests::test_helper::{LIQUID_STAKE_TOKEN_ADDRESS, NATIVE_TOKEN, UNION1, UNION2, UNION3, init},
    types::{Batch, BatchId, BatchState},
};

#[test]
fn submit_batch_works() {
    let mut deps = init();

    let _ = deps
        .storage
        .upsert_item::<AccountingStateStore, StdError>(|s| {
            let mut s = s.unwrap();
            s.total_bonded_native_tokens = 1_100;
            s.total_issued_lst = 1_000;
            Ok(s)
        })
        .unwrap();

    // UNION1 bonds 1000 tokens
    let union1_bond_amount = 1000u128.into();
    let _ = execute(
        deps.as_mut(),
        mock_env(),
        message_info(
            &Addr::unchecked(UNION1),
            &[Coin {
                denom: NATIVE_TOKEN.into(),
                amount: union1_bond_amount,
            }],
        ),
        ExecuteMsg::Bond {
            mint_to: Addr::unchecked(UNION1),
            min_mint_amount: 909u128.into(),
        },
    )
    .unwrap();

    // UNION2 bonds 2000 tokens
    let union2_bond_amount = 2000u128.into();
    let _ = execute(
        deps.as_mut(),
        mock_env(),
        message_info(
            &Addr::unchecked(UNION2),
            &[Coin {
                denom: NATIVE_TOKEN.into(),
                amount: union2_bond_amount,
            }],
        ),
        ExecuteMsg::Bond {
            mint_to: Addr::unchecked(UNION2),
            min_mint_amount: 1818u128.into(),
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
            staker: Addr::unchecked(UNION3),
            amount: union3_unbond_amount,
        },
    )
    .unwrap();

    let batch = deps
        .storage
        .read::<Batches>(&BatchId::from_raw(1).unwrap())
        .unwrap();

    let mut env = mock_env();
    env.block.time = if let BatchState::Pending { submit_time } = batch.state {
        Timestamp::from_seconds(submit_time + 1)
    } else {
        panic!("invalid state")
    };

    let res = execute(
        deps.as_mut(),
        env.clone(),
        message_info(&Addr::unchecked(UNION1), &[]),
        ExecuteMsg::SubmitBatch {},
    )
    .unwrap();

    let new_batch_id = BatchId::from_raw(2).unwrap();

    // latest batch id is increased
    assert_eq!(
        deps.storage.read_item::<PendingBatchId>().unwrap(),
        new_batch_id
    );

    // new pending batch is pushed to the batches
    let new_batch = deps.storage.read::<Batches>(&new_batch_id).unwrap();
    assert_eq!(
        new_batch,
        Batch::new_pending(
            env.block.time.seconds()
                + deps
                    .storage
                    .read_item::<ConfigStore>()
                    .unwrap()
                    .batch_period_seconds
        ),
    );

    let state = deps.storage.read_item::<AccountingStateStore>().unwrap();

    // the rate is updated properly
    assert_eq!(state.total_bonded_native_tokens, 3550);
    assert_eq!(state.total_issued_lst, 3227);

    let batch = deps
        .storage
        .read::<Batches>(&BatchId::from_raw(1).unwrap())
        .unwrap();

    // batch status is properly updated
    assert_eq!(
        batch.state,
        BatchState::Submitted {
            receive_time: env.block.time.seconds() + 100_000,
            expected_native_unstaked: 550
        }
    );

    // the given unbond amount is gonna be burned
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: LIQUID_STAKE_TOKEN_ADDRESS.into(),
            msg: to_json_binary(&Cw20ExecuteMsg::Burn {
                amount: 500u128.into()
            })
            .unwrap(),
            funds: vec![]
        })
    );

    // no further messages
    assert_eq!(res.messages.len(), 1);

    // event is emitted correctly
    assert_eq!(
        res.events[0],
        Event::new("submit_batch")
            .add_attribute("batch_id", "1")
            .add_attribute("batch_total", "500")
            .add_attribute("expected_unstaked", "550")
            .add_attribute("current_unbonding_period", "100000"),
    );
}

// #[test]
// fn empty_submit_batch() {
//     let mut deps = init();
//     let mut env = mock_env();

//     let state = STATE.load(&deps.storage).unwrap();
//     let config = CONFIG.load(&deps.storage).unwrap();

//     STATE.save(&mut deps.storage, &state).unwrap();

//     env.block.time = env.block.time.plus_seconds(config.batch_period + 1);
//     let msg = ExecuteMsg::SubmitBatch {};

//     let contract = env.contract.address.clone().to_string();

//     let info = mock_info(&contract, &[]);
//     let res = execute(deps.as_mut(), env, info, msg);
//     assert!(res.is_err());
// }

// #[test]
// fn not_ready_submit_batch() {
//     let mut deps = init();
//     let mut env = mock_env();

//     let mut state = STATE.load(&deps.storage).unwrap();
//     let config = CONFIG.load(&deps.storage).unwrap();

//     state.total_liquid_stake_token = Uint128::from(100_000u128);
//     state.total_bonded_native_tokens = Uint128::from(300_000u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     // batch isnt ready
//     env.block.time = env.block.time.plus_seconds(config.batch_period - 1);
//     let msg = ExecuteMsg::SubmitBatch {};

//     let contract = env.contract.address.clone().to_string();

//     let info = mock_info(&contract, &[]);
//     let res = execute(deps.as_mut(), env, info, msg);

//     assert!(res.is_err());
// }

// #[test]
// fn pending_batch_with_to_many_lst_tokens_fails() {
//     let mut deps = init();
//     let mut env = mock_env();

//     let mut state = STATE.load(&deps.storage).unwrap();
//     let config = CONFIG.load(&deps.storage).unwrap();

//     state.total_liquid_stake_token = Uint128::from(100_000u128);
//     state.total_bonded_native_tokens = Uint128::from(300_000u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     // Create a pending batch with to many tokens
//     PENDING_BATCH_ID.save(&mut deps.storage, &1).unwrap();
//     BATCHES
//         .save(
//             &mut deps.storage,
//             1,
//             &Batch {
//                 id: 1,
//                 unbond_requests_count: Some(1),
//                 batch_total_liquid_stake: Uint128::new(state.total_liquid_stake_token.u128() + 1),
//                 state: BatchState::Received,
//                 next_batch_action_time: Some(
//                     env.block.time.plus_seconds(config.batch_period).seconds(),
//                 ),
//                 liquid_unbond_requests: None,
//                 expected_native_unstaked: None,
//                 received_native_unstaked: None,
//             },
//         )
//         .unwrap();

//     // Update the time to simulate batch readiness.
//     env.block.time = env.block.time.plus_seconds(config.batch_period + 1);

//     let msg = ExecuteMsg::SubmitBatch {};
//     let info = mock_info(OSMO1, &[]);
//     let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

//     assert!(match err {
//         ContractError::InvalidUnstakeAmount { .. } => true,
//         _ => false,
//     })
// }
