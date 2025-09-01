use cosmwasm_std::{
    Addr, Attribute, BankMsg, Coin, CosmosMsg, StdError,
    testing::{message_info, mock_env},
};

use super::test_helper::UNION1;
use crate::{
    contract::execute,
    msg::ExecuteMsg,
    state::STATE,
    tests::test_helper::{NATIVE_TOKEN, UNION_STAKER, init, mock_init_msg},
};

#[test]
fn receive_rewards_works() {
    let mut deps = init();

    let state = STATE
        .update(&mut deps.storage, |mut s| {
            s.total_bonded_native_tokens = 100_000u128.into();
            s.total_issued_lst = 100_000u128.into();
            Ok::<_, StdError>(s)
        })
        .unwrap();

    let reward_amount = 100u128;
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(
            &Addr::unchecked(UNION1),
            &[Coin {
                denom: NATIVE_TOKEN.into(),
                amount: reward_amount.into(),
            }],
        ),
        ExecuteMsg::ReceiveRewards {},
    )
    .unwrap();

    // fee will be 10 because our protocol fee config is 10%
    let fee = 10u128;

    // amount - fee must be sent back to the staker
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: UNION_STAKER.into(),
            amount: vec![Coin {
                denom: NATIVE_TOKEN.into(),
                amount: (reward_amount - fee).into()
            }]
        })
    );

    // fee must be sent to the recipient
    assert_eq!(
        res.messages[1].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: mock_init_msg().protocol_fee_config.fee_recipient.into(),
            amount: vec![Coin {
                denom: NATIVE_TOKEN.into(),
                amount: fee.into()
            }]
        })
    );

    // the state must be updated correctly
    let new_state = STATE.load(&deps.storage).unwrap();
    assert_eq!(
        new_state.total_bonded_native_tokens.u128(),
        state.total_bonded_native_tokens.u128() + (reward_amount - fee)
    );
    assert_eq!(new_state.total_reward_amount.u128(), reward_amount);

    // the event must be emitted correctly
    assert_eq!(
        res.attributes,
        vec![
            Attribute::new("action", "receive_rewards"),
            Attribute::new("action", "transfer_stake"),
            Attribute::new("amount", reward_amount.to_string()),
            Attribute::new("amount_after_fees", (reward_amount - fee).to_string()),
        ]
    );
}

// #[test]
// fn receive_rewards_and_send_fees_to_treasury() {
//     let mut deps = init();
//     let env = mock_env();

//     let mut state = STATE.load(&deps.storage).unwrap();
//     let config = CONFIG.load(&deps.storage).unwrap();

//     state.total_liquid_stake_token = Uint128::from(100_000u128);
//     state.total_bonded_native_tokens = Uint128::from(100_000u128);
//     state.total_reward_amount = Uint128::from(0u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     let msg = ExecuteMsg::ReceiveRewards {};

//     let sender = derive_intermediate_sender(
//         &config.protocol_chain_config.ibc_channel_id,
//         config.native_chain_config.reward_collector_address.as_str(),
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
//     let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
//     assert_eq!(res.messages.len(), 3); // transfer message, redemption/purchase rate update and
//                                        // send message to treasury
//     assert_eq!(res.messages[1].reply_on, ReplyOn::Always);
//     assert_eq!(
//         res.messages[1].msg,
//         CosmosMsg::from(MsgTransfer {
//             source_channel: CHANNEL_ID.to_string(),
//             source_port: "transfer".to_string(),
//             sender: env.contract.address.to_string(),
//             receiver: Addr::unchecked(STAKER_ADDRESS).to_string(),
//             token: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
//                 denom: NATIVE_TOKEN.to_string(),
//                 amount: "90".to_string(),
//             }),
//             timeout_height: None,
//             timeout_timestamp: env.block.time.nanos() + IBC_TIMEOUT.nanos(),
//             memo: format!("{{\"ibc_callback\":\"{}\"}}", env.contract.address),
//         })
//     );
//     assert_eq!(
//         res.messages[2].msg,
//         CosmosMsg::from(cosmwasm_std::BankMsg::Send {
//             to_address: config
//                 .protocol_fee_config
//                 .fee_recipient
//                 .unwrap()
//                 .to_string(),
//             amount: vec![cosmwasm_std::Coin::new(10u128, NATIVE_TOKEN)],
//         })
//     );

//     let state = STATE.load(&deps.storage).unwrap();

//     assert_eq!(state.total_reward_amount, Uint128::from(100u128));
//     assert_eq!(state.total_bonded_native_tokens, Uint128::from(100_090u128));
//     assert_eq!(state.total_fees, Uint128::from(0u128));
// }

// #[test]
// fn receive_rewards_with_zero_fees_fails() {
//     let mut deps = init();
//     let env = mock_env();

//     let mut state = STATE.load(&deps.storage).unwrap();
//     let config = CONFIG.load(&deps.storage).unwrap();

//     state.total_liquid_stake_token = Uint128::from(100_000u128);
//     state.total_bonded_native_tokens = Uint128::from(100_000u128);
//     state.total_reward_amount = Uint128::from(0u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     let msg = ExecuteMsg::ReceiveRewards {};

//     let sender = derive_intermediate_sender(
//         &config.protocol_chain_config.ibc_channel_id,
//         config.native_chain_config.reward_collector_address.as_str(),
//         config.protocol_chain_config.account_address_prefix.as_str(),
//     )
//     .unwrap();

//     let info = mock_info(
//         &sender,
//         &[cosmwasm_std::Coin {
//             amount: Uint128::from(3u128),
//             denom: config.protocol_chain_config.native_token_denom.clone(),
//         }],
//     );
//     let err = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
//     assert!(match err {
//         ContractError::ComputedFeesAreZero { .. } => true,
//         _ => false,
//     });
// }
