use cosmwasm_std::{
    attr,
    testing::{message_info, mock_env, mock_info},
    to_json_binary, Addr, Attribute, Coin, CosmosMsg, Timestamp, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

use super::test_helper::{NATIVE_TOKEN, UNION1, UNION2, UNION3};
use crate::{
    contract::execute,
    error::ContractError,
    helpers::compute_unbond_amount,
    msg::ExecuteMsg,
    state::{BATCHES, CONFIG, PENDING_BATCH_ID, STATE},
    tests::test_helper::{init, LIQUID_STAKE_TOKEN_ADDRESS, OSMO1},
    types::{Batch, BatchState},
};

#[test]
fn submit_batch_works() {
    let mut deps = init();

    let mut state = STATE.load(&deps.storage).unwrap();

    let initial_total_native_token = Uint128::from(1_100u128);
    state.total_native_token = initial_total_native_token;

    let initial_total_liquid_stake_token = Uint128::from(1_000u128);
    state.total_bonded_lst = initial_total_liquid_stake_token;

    STATE.save(&mut deps.storage, &state).unwrap();

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
            mint_to: UNION2.as_bytes().to_vec().into(),
            recipient_channel_id: None,
            min_mint_amount: None,
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
            mint_to: UNION2.as_bytes().to_vec().into(),
            recipient_channel_id: None,
            min_mint_amount: None,
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
            staker: UNION3.to_string(),
            amount: union3_unbond_amount,
        },
    )
    .unwrap();

    let batch = BATCHES.load(&deps.storage, 1).unwrap();

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

    // latest batch id is increased
    assert_eq!(PENDING_BATCH_ID.load(&deps.storage).unwrap(), 2);

    // new pending batch is pushed to the batches
    let new_batch = BATCHES.load(&deps.storage, 2).unwrap();
    assert_eq!(
        new_batch,
        Batch::new_pending(
            env.block.time.seconds() + CONFIG.load(&deps.storage).unwrap().batch_period
        ),
    );

    let state = STATE.load(&deps.storage).unwrap();

    // the rate is updated properly
    let expected_total_native_token: Uint128 = 3550u128.into();
    assert_eq!(state.total_native_token, expected_total_native_token);

    let expected_total_liquid_token: Uint128 = 3227u128.into();
    assert_eq!(state.total_bonded_lst, expected_total_liquid_token);

    let batch = BATCHES.load(&deps.storage, 1).unwrap();

    // batch status is properly updated
    assert_eq!(
        batch.state,
        BatchState::Submitted {
            receive_time: env.block.time.seconds() + 100_000,
            expected_native_unstaked: 550u128.into()
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
        res.attributes,
        vec![
            attr("action", "submit_batch"),
            attr("batch_id", "1"),
            attr("batch_total", "500"),
            attr("expected_unstaked", "550"),
            attr("unbonding_period", "100000"),
        ]
    )
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
//     state.total_native_token = Uint128::from(300_000u128);
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
//     state.total_native_token = Uint128::from(300_000u128);
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
