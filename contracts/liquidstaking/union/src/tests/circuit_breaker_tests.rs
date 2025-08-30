use cosmwasm_std::{
    coins,
    testing::{mock_env, mock_info},
    Coin, Uint128,
};

use crate::{
    contract::execute,
    msg::{self, ExecuteMsg},
    state::{new_unstake_request, State, BATCHES, CONFIG, STATE},
    tests::test_helper::{init, ADMIN, NATIVE_TOKEN, OSMO2, OSMO3},
    types::Batch,
};

#[test]
fn circuit_breaker() {
    let mut deps = init();
    let mut env = mock_env();

    let mut state = STATE.load(&deps.storage).unwrap();
    let config = CONFIG.load(&deps.storage).unwrap();

    state.total_liquid_stake_token = Uint128::from(100_000u128);
    state.total_bonded_native_tokens = Uint128::from(300_000u128);
    STATE.save(&mut deps.storage, &state).unwrap();

    let msg = ExecuteMsg::CircuitBreaker {};

    let contract = env.contract.address.clone().to_string();

    // not correct sender
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info(&contract, &[]),
        msg.clone(),
    );

    assert!(res.is_err());

    // correct sender (admin)
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info(OSMO3, &[]),
        msg.clone(),
    );

    assert!(res.is_ok());

    // correct sender (operator)
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info(OSMO2, &[]),
        msg.clone(),
    );

    assert!(res.is_ok());

    // liquid stake
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(OSMO3, &coins(1000, "osmoTIA")).clone(),
        ExecuteMsg::Bond {
            mint_to: OSMO3.as_bytes().into(),
            recipient_channel_id: None,
            min_mint_amount: None,
        },
    );
    assert!(res.is_err());

    // liquid unstake
    state.total_liquid_stake_token = Uint128::from(100_000u128);
    state.total_bonded_native_tokens = Uint128::from(300_000u128);
    STATE.save(&mut deps.storage, &state).unwrap();
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("bob", &[]).clone(),
        ExecuteMsg::Unbond {
            staker: "bob".into(),
            amount: 1000.into(),
        },
    );
    assert!(res.is_err());

    // receive rewards
    let msg = ExecuteMsg::ReceiveRewards {};
    let sender = derive_intermediate_sender(
        &config.protocol_chain_config.ibc_channel_id,
        config.native_chain_config.reward_collector_address.as_ref(),
        &config.protocol_chain_config.account_address_prefix,
    )
    .unwrap();
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info(
            &sender,
            &[Coin {
                amount: Uint128::from(100u128),
                denom: config.protocol_chain_config.native_token_denom.clone(),
            }],
        ),
        msg.clone(),
    );
    assert!(res.is_err());

    // receive unstaked tokens
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info(
            &sender,
            &[Coin {
                amount: Uint128::from(100u128),
                denom: config.protocol_chain_config.native_token_denom.clone(),
            }],
        ),
        ExecuteMsg::ReceiveUnstakedTokens { batch_id: 1 }.clone(),
    );
    assert!(res.is_err());

    // execute withdraw
    let mut pending_batch: Batch =
        Batch::new_pending(Uint128::zero(), env.block.time.seconds() + 10000);
    new_unstake_request(
        &mut deps.as_mut(),
        "bob".to_string(),
        1,
        Uint128::from(10u128),
    )
    .unwrap();
    pending_batch.state = crate::types::BatchState::Received;
    BATCHES.save(&mut deps.storage, 1, &pending_batch).unwrap();
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info("bob", &[]),
        ExecuteMsg::Withdraw { batch_id: 1 }.clone(),
    );
    assert!(res.is_err());

    // submit batch
    env.block.time = env.block.time.plus_seconds(config.batch_period - 1);
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info(&contract, &[]),
        ExecuteMsg::SubmitBatch {}.clone(),
    );
    assert!(res.is_err());

    // reenable
    let msg = ExecuteMsg::ResumeContract {
        total_bonded_native_tokens: Uint128::from(100000u128),
        total_liquid_stake_token: Uint128::from(200000u128),
        total_reward_amount: Uint128::from(10000u128),
    };

    // not correct sender
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info(&contract, &[]),
        msg.clone(),
    );

    assert!(res.is_err());

    // correct sender
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info(ADMIN, &[]),
        msg.clone(),
    );

    assert!(res.is_ok());

    // test accounting update
    let state: State = STATE.load(&deps.storage).unwrap();
    assert_eq!(state.total_liquid_stake_token, Uint128::from(200000u128));
    assert_eq!(state.total_bonded_native_tokens, Uint128::from(100000u128));
    assert_eq!(state.total_reward_amount, Uint128::from(10000u128));

    // test can't resume contract
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info(OSMO3, &[]),
        msg.clone(),
    );
    assert!(res.is_err());

    // test enabled
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(OSMO3, &coins(1000, NATIVE_TOKEN)).clone(),
        ExecuteMsg::Bond {
            min_mint_amount: None,
            transfer_to_native_chain: None,
            mint_to: None,
        },
    );
    assert!(res.is_ok());
}
