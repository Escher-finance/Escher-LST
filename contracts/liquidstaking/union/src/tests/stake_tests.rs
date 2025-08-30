use alloy::{primitives::U256, sol_types::SolValue};
use cosmwasm_std::{
    attr, coins,
    testing::{message_info, mock_env},
    to_json_binary, Addr, BankMsg, CosmosMsg, StdError, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use ibc_union_spec::ChannelId;
use ucs03_zkgm::com::{
    Instruction, TokenOrderV2, INSTR_VERSION_2, OP_TOKEN_ORDER, TOKEN_ORDER_KIND_SOLVE,
};
use unionlabs_primitives::{encoding::HexPrefixed, Bytes};

use crate::{
    contract::execute,
    msg::ExecuteMsg,
    state::{CounterpartyConfig, FUNGIBLE_RECIPIENT_CHANNEL, STATE},
    tests::test_helper::{
        init, FUNDED_DISPATCH_ADDRESS, LIQUID_STAKE_TOKEN_ADDRESS, NATIVE_TOKEN, UNION2,
        UNION_STAKER, ZKGM_ADDRESS,
    },
};

#[test]
fn bond_with_local_recipient_works() {
    let mut deps = init();
    let info = message_info(
        &Addr::unchecked(FUNDED_DISPATCH_ADDRESS),
        &coins(1000, NATIVE_TOKEN),
    );
    let msg = ExecuteMsg::Bond {
        mint_to: UNION2.to_string().as_bytes().into(),
        recipient_channel_id: None,
        min_mint_amount: None,
    };

    let mut prev_state = STATE.load(&deps.storage).unwrap();

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // the native funds are sent to the staker
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: UNION_STAKER.into(),
            amount: info.funds.clone()
        })
    );

    // 1000 LST token is minted to the `mint_to` address.
    // the `mint_amount` is 1000, since no rewards have been processed yet.
    assert_eq!(
        res.messages[1].msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: LIQUID_STAKE_TOKEN_ADDRESS.into(),
            msg: to_json_binary(&Cw20ExecuteMsg::Mint {
                recipient: UNION2.into(),
                amount: 1000u128.into()
            })
            .unwrap(),
            funds: vec![]
        })
    );

    // there should be no further messages
    assert_eq!(res.messages.len(), 2);

    // the event is correct
    assert_eq!(
        res.attributes,
        vec![
            attr(
                "mint_to_address",
                <Bytes>::new(UNION2.as_bytes().to_vec()).to_string()
            ),
            attr("action", "bond"),
            attr("sender", FUNDED_DISPATCH_ADDRESS),
            attr("in_amount", 1000.to_string()),
            attr("mint_amount", 1000.to_string()),
        ]
    );

    let state = STATE.load(&deps.storage).unwrap();

    // state is properly adjusted
    assert_eq!(state.total_bonded_native_tokens.u128(), 1000);
    assert_eq!(state.total_issued_lst.u128(), 1000);

    prev_state.total_bonded_native_tokens = 1000u128.into();
    prev_state.total_issued_lst = 1000u128.into();

    // there is no further state change
    assert_eq!(state, prev_state);

    // manually changing the rate instead of going through the `rewards` entrypoint
    STATE
        .update::<_, StdError>(&mut deps.storage, |mut s| {
            s.total_bonded_native_tokens += Uint128::new(100);
            Ok(s)
        })
        .unwrap();

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Now, since the rate is 1000/1100, we should mint 909 lst tokens for 1000 U's.
    assert_eq!(
        res.messages[1].msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: LIQUID_STAKE_TOKEN_ADDRESS.into(),
            msg: to_json_binary(&Cw20ExecuteMsg::Mint {
                recipient: UNION2.into(),
                amount: 909u128.into()
            })
            .unwrap(),
            funds: vec![]
        })
    );
}
#[test]
fn bond_with_remote_recipient_works() {
    let mut deps = init();
    let env = mock_env();
    let info = message_info(
        &Addr::unchecked(FUNDED_DISPATCH_ADDRESS),
        &coins(1000, NATIVE_TOKEN),
    );
    let msg = ExecuteMsg::Bond {
        mint_to: UNION2.to_string().as_bytes().into(),
        recipient_channel_id: Some(ChannelId!(1)),
        min_mint_amount: None,
    };

    let counterparty_config = CounterpartyConfig {
        quote_token: b"quote_token".to_vec().into(),
        kind: TOKEN_ORDER_KIND_SOLVE,
        metadata: b"metadata".to_vec().into(),
    };

    // setting the fungible recipient so that we can send tokens through zkgm
    FUNGIBLE_RECIPIENT_CHANNEL
        .save(&mut deps.storage, 1, &counterparty_config)
        .unwrap();

    // starting with a rate that is different than 1
    STATE
        .update(&mut deps.storage, |mut s| {
            s.total_bonded_native_tokens += Uint128::new(1100);
            s.total_issued_lst += Uint128::new(1000);
            Ok::<_, StdError>(s)
        })
        .unwrap();

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    let mint_amount: Uint128 = 909u128.into();

    // the native funds are sent to the staker
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: UNION_STAKER.into(),
            amount: info.funds.clone()
        })
    );

    // 909 LST token is minted for 1000 U tokens to this contract to be sent through zkgm
    assert_eq!(
        res.messages[1].msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: LIQUID_STAKE_TOKEN_ADDRESS.into(),
            msg: to_json_binary(&Cw20ExecuteMsg::Mint {
                recipient: env.contract.address.into(),
                amount: mint_amount
            })
            .unwrap(),
            funds: vec![]
        })
    );

    // allowing zkgm to spend 909 LST tokens on behalf of this contract
    assert_eq!(
        res.messages[2].msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: LIQUID_STAKE_TOKEN_ADDRESS.into(),
            msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                spender: ZKGM_ADDRESS.into(),
                amount: mint_amount,
                expires: None
            })
            .unwrap(),
            funds: vec![]
        })
    );

    let amount = U256::from(mint_amount.u128());

    // sending the minted tokens through zkgm
    assert_eq!(
        res.messages[3].msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: ZKGM_ADDRESS.into(),
            msg: to_json_binary(&ucs03_zkgm::msg::ExecuteMsg::Send {
                channel_id: ChannelId!(1),
                timeout_height: 0u64.into(),
                timeout_timestamp: ibc_union_spec::Timestamp::from_nanos(
                    mock_env().block.time.plus_days(3).nanos()
                ),
                salt: Default::default(),
                instruction: Instruction {
                    // version 2 since we are using the solver api
                    version: INSTR_VERSION_2,
                    opcode: OP_TOKEN_ORDER,
                    operand: TokenOrderV2 {
                        // union staker as a fallback address
                        sender: UNION_STAKER.to_string().into_bytes().into(),
                        receiver: UNION2.to_string().into_bytes().into(),
                        base_token: LIQUID_STAKE_TOKEN_ADDRESS.as_bytes().to_vec().into(),
                        base_amount: amount,
                        quote_token: counterparty_config.quote_token.into(),
                        quote_amount: amount,
                        kind: counterparty_config.kind,
                        metadata: counterparty_config.metadata.into()
                    }
                    .abi_encode_params()
                    .into()
                }
                .abi_encode_params()
                .into()
            })
            .unwrap(),
            funds: vec![]
        })
    );

    // there should be no further messages
    assert_eq!(res.messages.len(), 4);

    // the event is correct
    assert_eq!(
        res.attributes,
        vec![
            attr(
                "mint_to_address",
                Bytes::<HexPrefixed>::new(UNION2.as_bytes().to_vec()).to_string()
            ),
            attr("action", "bond"),
            attr("sender", FUNDED_DISPATCH_ADDRESS),
            attr("in_amount", 1000.to_string()),
            attr("mint_amount", 909.to_string()),
        ]
    );

    let state = STATE.load(&deps.storage).unwrap();

    // state is properly adjusted
    assert_eq!(state.total_bonded_native_tokens.u128(), 2100);
    assert_eq!(state.total_issued_lst.u128(), 1909);
}

// #[test]
// fn proper_liquid_stake_with_ibc_transfer() {
//     let mut deps = init();
//     let env = mock_env();
//     let info = mock_info(OSMO3, &coins(1000, NATIVE_TOKEN));
//     let msg = ExecuteMsg::Bond {
//         mint_to: Some(CELESTIA2.to_string()),
//         transfer_to_native_chain: None,
//         min_mint_amount: None,
//     };
//     let res = execute(deps.as_mut(), mock_env(), info, msg.clone());

//     let timeout = IbcTimeout::with_timestamp(Timestamp::from_nanos(
//         env.block.time.nanos() + IBC_TIMEOUT.nanos(),
//     ));

//     let ibc_coin = Coin {
//         denom: NATIVE_TOKEN.to_string(),
//         amount: "1000".to_string(),
//     };

//     let ibc_sub_msg_id = env.block.time.nanos() + env.transaction.unwrap().index as u64;
//     match res {
//         Ok(ref result) => {
//             assert_eq!(
//                 result.attributes,
//                 vec![
//                     attr("action", "liquid_stake"),
//                     attr("sender", OSMO3),
//                     attr("in_amount", "1000"),
//                     attr("mint_amount", "1000"),
//                 ]
//             );
//             assert_eq!(result.messages.len(), 4); // mint, redemption rate update, stake IBC transfer, IBC transfer

//             // First message mints the liquid staked representation to the contract
//             assert_eq!(
//                 result.messages[0],
//                 SubMsg {
//                     id: 0,
//                     msg: <MsgMint as Into<CosmosMsg>>::into(MsgMint {
//                         sender: MOCK_CONTRACT_ADDR.to_string(),
//                         amount: Some(Coin {
//                             denom: format!("factory/cosmos2contract/{}", LIQUID_STAKE_TOKEN_DENOM),
//                             amount: "1000".to_string(),
//                         }),
//                         mint_to_address: MOCK_CONTRACT_ADDR.to_string(),
//                     }),
//                     gas_limit: None,
//                     reply_on: ReplyOn::Never,
//                 }
//             );

//             // The third message IBC transfer the staked tokens to the
//             // native chain to be staked.
//             assert_eq!(
//                 result.messages[2],
//                 SubMsg {
//                     id: ibc_sub_msg_id,
//                     msg: <MsgTransfer as Into<CosmosMsg>>::into(MsgTransfer {
//                         source_channel: CHANNEL_ID.to_string(),
//                         source_port: "transfer".to_string(),
//                         sender: env.contract.address.to_string(),
//                         receiver: Addr::unchecked(STAKER_ADDRESS).to_string(),
//                         token: Some(ibc_coin),
//                         timeout_height: None,
//                         timeout_timestamp: timeout.timestamp().unwrap().nanos(),
//                         memo: format!("{{\"ibc_callback\":\"{}\"}}", env.contract.address),
//                     }),
//                     gas_limit: None,
//                     reply_on: ReplyOn::Always,
//                 }
//             );

//             // The fourth message IBC transfer the minted liquid staking representation
//             // to the user.
//             assert_eq!(
//                 result.messages[3],
//                 SubMsg {
//                     id: ibc_sub_msg_id + 1,
//                     msg: <MsgTransfer as Into<CosmosMsg>>::into(MsgTransfer {
//                         source_channel: CHANNEL_ID.to_string(),
//                         source_port: "transfer".to_string(),
//                         sender: env.contract.address.to_string(),
//                         receiver: CELESTIA2.to_string(),
//                         token: Some(Coin {
//                             amount: "1000".to_string(),
//                             denom: format!("factory/cosmos2contract/{}", LIQUID_STAKE_TOKEN_DENOM),
//                         }),
//                         timeout_height: None,
//                         timeout_timestamp: timeout.timestamp().unwrap().nanos(),
//                         memo: format!("{{\"ibc_callback\":\"{}\"}}", env.contract.address),
//                     }),
//                     gas_limit: None,
//                     reply_on: ReplyOn::Always,
//                 }
//             );
//         }
//         Err(e) => {
//             panic!("Unexpected error: {:?}", e);
//         }
//     }

//     // need to do this or we can't send more ibc messages
//     // IBC_WAITING_FOR_REPLY.remove(deps.as_mut().storage);
//     reply(
//         deps.as_mut(),
//         mock_env(),
//         Reply {
//             id: ibc_sub_msg_id,
//             result: SubMsgResult::Ok(SubMsgResponse {
//                 data: Some(cosmwasm_std::Binary(Vec::new())), // No data returned
//                 events: Vec::new(),                           // No events
//             }),
//         },
//     )
//     .unwrap();
//     reply(
//         deps.as_mut(),
//         mock_env(),
//         Reply {
//             id: ibc_sub_msg_id + 1,
//             result: SubMsgResult::Ok(SubMsgResponse {
//                 data: Some(cosmwasm_std::Binary(Vec::new())), // No data returned
//                 events: Vec::new(),                           // No events
//             }),
//         },
//     )
//     .unwrap();

//     let pending_batch = BATCHES
//         .range(deps.as_ref().storage, None, None, Order::Descending)
//         .find(|r| r.is_ok() && r.as_ref().unwrap().1.state == BatchState::Pending)
//         .unwrap()
//         .unwrap()
//         .1;
//     assert!(pending_batch.id == 1);

//     // Use the previously unwrapped value
//     let state = STATE.load(deps.as_ref().storage).unwrap();
//     assert_eq!(state.total_liquid_stake_token, Uint128::from(1000u128));
//     assert_eq!(state.total_bonded_native_tokens, Uint128::from(1000u128));

//     let info = mock_info(OSMO3, &coins(10000, NATIVE_TOKEN));
//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
//     res.unwrap();

//     let state_for_osmo3 = STATE.load(&deps.storage).unwrap();
//     assert_eq!(
//         state_for_osmo3.total_liquid_stake_token,
//         Uint128::from(11000u128)
//     );
//     assert_eq!(state_for_osmo3.total_bonded_native_tokens, Uint128::from(11000u128));

//     // set total_liquid_stake_token: 1_000_000_000,
//     // native_token: 1_000_000
//     deps = init();
//     let mut state = STATE.load(&deps.storage).unwrap();
//     state.total_liquid_stake_token = Uint128::from(1_000_000_000u128);
//     state.total_bonded_native_tokens = Uint128::from(1_000_000u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     let info = mock_info(OSMO3, &coins(50_000_000, NATIVE_TOKEN));
//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
//     assert!(res.is_ok());

//     let state = STATE.load(&deps.storage).unwrap();
//     assert_eq!(
//         state.total_liquid_stake_token,
//         Uint128::from(51_000_000_000u128)
//     );
//     assert_eq!(state.total_bonded_native_tokens, Uint128::from(51_000_000u128));

//     // set total_liquid_stake_token: 1_000_000,
//     // native_token: 1_000_000_000
//     deps = init();
//     let mut state = STATE.load(&deps.storage).unwrap();
//     state.total_liquid_stake_token = Uint128::from(1_000_000u128);
//     state.total_bonded_native_tokens = Uint128::from(1_000_000_000u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     let info = mock_info(OSMO3, &coins(50_000_000, NATIVE_TOKEN));
//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
//     assert!(res.is_ok());

//     let state = STATE.load(&deps.storage).unwrap();
//     assert_eq!(state.total_liquid_stake_token, Uint128::from(1_050_000u128));
//     assert_eq!(state.total_bonded_native_tokens, Uint128::from(1_050_000_000u128));

//     // test redemption rate, purchase rate
//     let (redemption_rate, purchase_rate) = get_rates(&state);
//     assert_eq!(
//         redemption_rate,
//         Decimal::from_ratio(1_050_000_000u128, 1_050_000u128)
//     );
//     assert_eq!(
//         purchase_rate,
//         Decimal::from_ratio(1_050_000u128, 1_050_000_000u128)
//     );
// }

// #[test]
// fn liquid_stake_less_than_minimum() {
//     let mut deps = init();
//     let info = mock_info(OSMO3, &coins(10, NATIVE_TOKEN));
//     let msg = ExecuteMsg::Bond {
//         mint_to: None,
//         transfer_to_native_chain: None,
//         min_mint_amount: None,
//     };

//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
//     match res {
//         Ok(_) => panic!("Expected error"),
//         Err(e) => {
//             if let ContractError::MinimumLiquidStakeAmount {
//                 minimum_stake_amount,
//                 sent_amount,
//             } = e
//             {
//                 assert_eq!(minimum_stake_amount, Uint128::from(100u128));
//                 assert_eq!(sent_amount, Uint128::from(10u128));
//             } else {
//                 panic!("Unexpected error: {:?}", e);
//             }
//         }
//     }
// }

// #[test]
// fn proper_ibc_liquid_stake() {
//     let mut deps = init();
//     let intermediate_sender = derive_intermediate_sender(CHANNEL_ID, CELESTIA1, "osmo").unwrap();

//     let info = mock_info(&intermediate_sender, &coins(1000, NATIVE_TOKEN));
//     let msg: ExecuteMsg = ExecuteMsg::Bond {
//         mint_to: Some(OSMO3.to_string()),
//         transfer_to_native_chain: None,
//         min_mint_amount: None,
//     };

//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
//     if res.is_err() {
//         panic!("Unexpected error: {:?}", res);
//     }
//     assert!(res.is_ok());
// }

// #[test]
// fn receive_rewards_before_minting() {
//     let mut deps = init();
//     let env = mock_env();

//     let config = CONFIG.load(&deps.storage).unwrap();

//     // received rewards in advance of any liquid stake requests
//     let sender = derive_intermediate_sender(
//         &config.protocol_chain_config.ibc_channel_id,
//         config.native_chain_config.reward_collector_address.as_str(),
//         config.native_chain_config.account_address_prefix.as_str(),
//     )
//     .unwrap();
//     let resp = execute(
//         deps.as_mut(),
//         env.clone(),
//         mock_info(&sender, &coins(1_000, NATIVE_TOKEN)),
//         ExecuteMsg::ReceiveRewards {},
//     );

//     assert!(resp.is_err());
// }
// #[test]
// fn mint_amount_divergence() {
//     let mut deps = init();
//     let mut state: State = STATE.load(&deps.storage).unwrap();
//     state.total_liquid_stake_token = Uint128::from(1_000_000_000u128);
//     state.total_bonded_native_tokens = Uint128::from(1_000_000u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     let info = mock_info(OSMO3, &coins(1000, NATIVE_TOKEN));
//     let msg = ExecuteMsg::Bond {
//         mint_to: None,
//         transfer_to_native_chain: None,
//         min_mint_amount: Some(Uint128::from(2_000_000u128)),
//     };
//     let res: Result<cosmwasm_std::Response, ContractError> =
//         execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
//     assert!(res.is_err()); // minted amount is lower than expected

//     let msg = ExecuteMsg::Bond {
//         mint_to: None,
//         transfer_to_native_chain: None,
//         min_mint_amount: Some(Uint128::from(1_000_000u128)),
//     };
//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
//     if res.is_err() {
//         panic!("Unexpected error: {:?}", res);
//     }
//     assert!(res.is_ok());
// }

// #[test]
// fn zero_liquid_stake_but_native_tokens() {
//     let mut deps = init();

//     let mut state: State = STATE.load(&deps.storage).unwrap();
//     state.total_bonded_native_tokens = Uint128::from(1000u128);
//     state.total_liquid_stake_token = Uint128::from(0u128);
//     state.total_fees = Uint128::from(100u128);
//     STATE.save(&mut deps.storage, &state).unwrap();

//     let info = mock_info(OSMO3, &coins(1000, NATIVE_TOKEN));
//     let msg = ExecuteMsg::Bond {
//         mint_to: None,
//         transfer_to_native_chain: None,
//         min_mint_amount: None,
//     };
//     let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
//     assert!(res.is_ok());

//     let state: State = STATE.load(&deps.storage).unwrap();
//     assert_eq!(state.total_bonded_native_tokens, Uint128::from(1000u128));
//     assert_eq!(state.total_liquid_stake_token, Uint128::from(1000u128));
//     assert_eq!(state.total_fees, Uint128::from(1100u128));
// }

// #[test]
// fn transfer_to_native_chain_false_is_handle_correctly() {
//     let mut deps = init();
//     let env = mock_env();
//     // The flag is handled only when native and protocol chain address prefix
//     // are equal.
//     CONFIG
//         .update::<_, StdError>(&mut deps.storage, |mut c| {
//             c.native_chain_config.account_address_prefix = "osmo".to_string();
//             c.protocol_chain_config.account_address_prefix = "osmo".to_string();
//             Ok(c)
//         })
//         .unwrap();

//     let info = mock_info(OSMO3, &coins(1000, NATIVE_TOKEN));
//     let msg = ExecuteMsg::Bond {
//         mint_to: None,
//         transfer_to_native_chain: None,
//         min_mint_amount: None,
//     };
//     let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);

//     let timeout = IbcTimeout::with_timestamp(Timestamp::from_nanos(
//         env.block.time.nanos() + IBC_TIMEOUT.nanos(),
//     ));

//     let ibc_coin = Coin {
//         denom: NATIVE_TOKEN.to_string(),
//         amount: "1000".to_string(),
//     };

//     let ibc_sub_msg_id = env.block.time.nanos() + env.transaction.unwrap().index as u64;
//     match res {
//         Ok(ref result) => {
//             assert_eq!(
//                 result.attributes,
//                 vec![
//                     attr("action", "liquid_stake"),
//                     attr("sender", OSMO3),
//                     attr("in_amount", "1000"),
//                     attr("mint_amount", "1000"),
//                 ]
//             );
//             assert_eq!(result.messages.len(), 4); // transfer, mint, redemption rate update

//             // First message mints the liquid staked representation to the contract
//             assert_eq!(
//                 result.messages[0],
//                 SubMsg {
//                     id: 0,
//                     msg: <MsgMint as Into<CosmosMsg>>::into(MsgMint {
//                         sender: MOCK_CONTRACT_ADDR.to_string(),
//                         amount: Some(Coin {
//                             denom: format!("factory/cosmos2contract/{}", LIQUID_STAKE_TOKEN_DENOM),
//                             amount: "1000".to_string(),
//                         }),
//                         mint_to_address: MOCK_CONTRACT_ADDR.to_string(),
//                     }),
//                     gas_limit: None,
//                     reply_on: ReplyOn::Never,
//                 }
//             );

//             // The third message IBC transfer the staked tokens to the
//             // native chain to be staked.
//             assert_eq!(
//                 result.messages[2],
//                 SubMsg {
//                     id: ibc_sub_msg_id.clone(),
//                     msg: <MsgTransfer as Into<CosmosMsg>>::into(MsgTransfer {
//                         source_channel: CHANNEL_ID.to_string(),
//                         source_port: "transfer".to_string(),
//                         sender: env.contract.address.to_string(),
//                         receiver: Addr::unchecked(STAKER_ADDRESS).to_string(),
//                         token: Some(ibc_coin),
//                         timeout_height: None,
//                         timeout_timestamp: timeout.timestamp().unwrap().nanos(),
//                         memo: format!("{{\"ibc_callback\":\"{}\"}}", env.contract.address),
//                     }),
//                     gas_limit: None,
//                     reply_on: ReplyOn::Always,
//                 }
//             );

//             // The fourth message sends the minted liquid staking representation
//             // to the user.
//             assert_eq!(
//                 result.messages[3],
//                 SubMsg {
//                     id: 0,
//                     msg: <MsgSend as Into<CosmosMsg>>::into(MsgSend {
//                         from_address: Addr::unchecked(MOCK_CONTRACT_ADDR).to_string(),
//                         to_address: OSMO3.to_string(),
//                         amount: vec![Coin {
//                             denom: format!("factory/cosmos2contract/{}", LIQUID_STAKE_TOKEN_DENOM),
//                             amount: "1000".to_string(),
//                         }],
//                     }),
//                     gas_limit: None,
//                     reply_on: ReplyOn::Never,
//                 }
//             );
//         }
//         Err(e) => {
//             panic!("Unexpected error: {:?}", e);
//         }
//     }
// }

// #[test]
// fn transfer_to_native_chain_true_is_handle_correctly() {
//     let mut deps = init();
//     let env = mock_env();
//     // The flag is handled only when native and protocol chain address prefix
//     // are equal.
//     CONFIG
//         .update::<_, StdError>(&mut deps.storage, |mut c| {
//             c.native_chain_config.account_address_prefix = "osmo".to_string();
//             c.protocol_chain_config.account_address_prefix = "osmo".to_string();
//             Ok(c)
//         })
//         .unwrap();

//     let info = mock_info(OSMO3, &coins(1000, NATIVE_TOKEN));
//     let msg = ExecuteMsg::Bond {
//         mint_to: None,
//         transfer_to_native_chain: Some(true),
//         min_mint_amount: None,
//     };
//     let res = execute(deps.as_mut(), env.clone(), info, msg);

//     let timeout = IbcTimeout::with_timestamp(Timestamp::from_nanos(
//         env.block.time.nanos() + IBC_TIMEOUT.nanos(),
//     ));

//     let ibc_coin = Coin {
//         denom: NATIVE_TOKEN.to_string(),
//         amount: "1000".to_string(),
//     };

//     let ibc_sub_msg_id = env.block.time.nanos() + env.transaction.unwrap().index as u64;
//     match res {
//         Ok(ref result) => {
//             assert_eq!(
//                 result.attributes,
//                 vec![
//                     attr("action", "liquid_stake"),
//                     attr("sender", OSMO3),
//                     attr("in_amount", "1000"),
//                     attr("mint_amount", "1000"),
//                 ]
//             );
//             assert_eq!(result.messages.len(), 4); // transfer, mint, redemption rate update

//             // First message mints the liquid staked representation to the contract
//             assert_eq!(
//                 result.messages[0],
//                 SubMsg {
//                     id: 0,
//                     msg: <MsgMint as Into<CosmosMsg>>::into(MsgMint {
//                         sender: MOCK_CONTRACT_ADDR.to_string(),
//                         amount: Some(Coin {
//                             denom: format!("factory/cosmos2contract/{}", LIQUID_STAKE_TOKEN_DENOM),
//                             amount: "1000".to_string(),
//                         }),
//                         mint_to_address: MOCK_CONTRACT_ADDR.to_string(),
//                     }),
//                     gas_limit: None,
//                     reply_on: ReplyOn::Never,
//                 }
//             );

//             // The third message IBC transfer the staked tokens to the
//             // native chain to be staked.
//             assert_eq!(
//                 result.messages[2],
//                 SubMsg {
//                     id: ibc_sub_msg_id,
//                     msg: <MsgTransfer as Into<CosmosMsg>>::into(MsgTransfer {
//                         source_channel: CHANNEL_ID.to_string(),
//                         source_port: "transfer".to_string(),
//                         sender: env.contract.address.to_string(),
//                         receiver: Addr::unchecked(STAKER_ADDRESS).to_string(),
//                         token: Some(ibc_coin),
//                         timeout_height: None,
//                         timeout_timestamp: timeout.timestamp().unwrap().nanos(),
//                         memo: format!("{{\"ibc_callback\":\"{}\"}}", env.contract.address),
//                     }),
//                     gas_limit: None,
//                     reply_on: ReplyOn::Always,
//                 }
//             );

//             // The fourth message sends the minted liquid staking representation
//             // to the user on the native chain.
//             assert_eq!(
//                 result.messages[3],
//                 SubMsg {
//                     id: ibc_sub_msg_id + 1,
//                     msg: <MsgTransfer as Into<CosmosMsg>>::into(MsgTransfer {
//                         source_channel: CHANNEL_ID.to_string(),
//                         source_port: "transfer".to_string(),
//                         token: Some(Coin {
//                             denom: format!("factory/cosmos2contract/{}", LIQUID_STAKE_TOKEN_DENOM),
//                             amount: "1000".to_string(),
//                         }),
//                         sender: env.contract.address.to_string(),
//                         receiver: OSMO3.to_string(),
//                         timeout_height: None,
//                         timeout_timestamp: timeout.timestamp().unwrap().nanos(),
//                         memo: format!("{{\"ibc_callback\":\"{}\"}}", env.contract.address),
//                     }),
//                     gas_limit: None,
//                     reply_on: ReplyOn::Always,
//                 }
//             );
//         }
//         Err(e) => {
//             panic!("Unexpected error: {:?}", e);
//         }
//     }
// }
