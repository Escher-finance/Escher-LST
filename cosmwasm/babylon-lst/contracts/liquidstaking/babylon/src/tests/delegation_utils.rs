#![allow(clippy::too_many_arguments, clippy::too_many_lines)]
use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{
    assert_approx_eq, from_json,
    testing::{mock_dependencies, mock_env, MockQuerier},
    Addr, AnyMsg, Attribute, Coin, CosmosMsg, DecCoin, Decimal, Decimal256, Empty, QuerierWrapper,
    StakingMsg, Timestamp, Uint128,
};
use cw_multi_test::{App, IntoAddr};
use prost::Message;

use crate::{
    event::{SUBMIT_BATCH_EVENT, UNBOND_EVENT, UNSTAKE_REQUEST_EVENT},
    execute::remote_bond,
    msg::{BondData, DelegationDiff, ValidatorDelegation},
    proto,
    state::{
        unbond_record, BurnQueue, MintQueue, State, SupplyQueue, UnbondRecord, Validator,
        ValidatorsRegistry, WithdrawRewardQueue, PARAMETERS, PENDING_BATCH_ID, REWARD_BALANCE,
        STATE, STATUS, SUPPLY_QUEUE, TOKEN_COUNT, WITHDRAW_REWARD_QUEUE,
    },
    tests::{mock_parameters, setup_validators_delegation, NATIVE_DENOM},
    utils::{
        batch::{batches, Batch, BatchStatus},
        calc::{self, normalize_supply_queue, normalize_total_supply},
        delegation::*,
    },
};

#[test]
fn test_get_validator_delegation_map_base_on_weight() {
    let validators = vec![
        Validator {
            address: "abc".to_string(),
            weight: 10,
        },
        Validator {
            address: "bcd".to_string(),
            weight: 20,
        },
    ];
    let total_delegated_amount = Uint128::new(250_000);
    let map =
        get_validator_delegation_map_base_on_weight(validators.clone(), total_delegated_amount);
    assert_eq!(map.len(), 2);
    assert_approx_eq!(
        map.values().sum::<Uint128>(),
        total_delegated_amount,
        "0.0001"
    );
    assert_approx_eq!(
        map.get("abc").unwrap() * Uint128::new(2),
        *map.get("bcd").unwrap(),
        "0.0001"
    );

    assert_eq!(
        HashMap::new(),
        get_validator_delegation_map_base_on_weight(vec![], total_delegated_amount)
    );
    assert_eq!(
        HashMap::from([
            (validators[0].address.clone(), Uint128::zero()),
            (validators[1].address.clone(), Uint128::zero())
        ]),
        get_validator_delegation_map_base_on_weight(validators, Uint128::zero())
    );
}

#[test]
fn test_get_surplus_deficit_validators() {
    let validator_a = "a".to_string();
    let validator_b = "b".to_string();
    let validator_c = "c".to_string();
    let validator_d = "d".to_string();
    let amount_a = Uint128::new(100);
    let amount_b = Uint128::new(1000);
    let amount_c = Uint128::new(250);
    let correct_amount_b = Uint128::new(800);
    let correct_amount_c = Uint128::new(350);
    let correct_amount_d = Uint128::new(500);
    let validator_delegation_map = HashMap::from([
        (validator_a.clone(), amount_a),
        (validator_b.clone(), amount_b),
        (validator_c.clone(), amount_c),
    ]);
    let correct_validator_delegation_map = HashMap::from([
        (validator_b.clone(), correct_amount_b),
        (validator_c.clone(), correct_amount_c),
        (validator_d.clone(), correct_amount_d),
    ]);
    let (mut surplus_validators, mut deficit_validators) =
        get_surplus_deficit_validators(validator_delegation_map, correct_validator_delegation_map);
    surplus_validators.sort_by_key(|v| v.address.clone());
    deficit_validators.sort_by_key(|v| v.address.clone());
    assert_eq!(
        surplus_validators,
        vec![
            ValidatorDelegation {
                address: validator_a.clone(),
                delegation_diff: DelegationDiff::Surplus,
                diff_amount: amount_a,
            },
            ValidatorDelegation {
                address: validator_b.clone(),
                delegation_diff: DelegationDiff::Surplus,
                diff_amount: amount_b.abs_diff(correct_amount_b),
            },
        ]
    );
    assert_eq!(
        deficit_validators,
        vec![
            ValidatorDelegation {
                address: validator_c.clone(),
                delegation_diff: DelegationDiff::Deficit,
                diff_amount: amount_c.abs_diff(correct_amount_c),
            },
            ValidatorDelegation {
                address: validator_d.clone(),
                delegation_diff: DelegationDiff::Deficit,
                diff_amount: correct_amount_d,
            },
        ]
    );
}

#[test]
fn test_get_restaking_msgs() {
    let delegator = "delegator".to_string();
    let surplus_validators = Vec::from([
        ValidatorDelegation {
            address: "a".to_string(),
            delegation_diff: DelegationDiff::Surplus,
            diff_amount: Uint128::new(200),
        },
        ValidatorDelegation {
            address: "b".to_string(),
            delegation_diff: DelegationDiff::Surplus,
            diff_amount: Uint128::new(900),
        },
    ]);
    let deficient_validators = vec![
        ValidatorDelegation {
            address: "c".to_string(),
            delegation_diff: DelegationDiff::Deficit,
            diff_amount: Uint128::new(1000),
        },
        ValidatorDelegation {
            address: "d".to_string(),
            delegation_diff: DelegationDiff::Deficit,
            diff_amount: Uint128::new(500),
        },
    ];
    let msgs = get_restaking_msgs(
        delegator.clone(),
        surplus_validators,
        deficient_validators,
        "denom".to_string(),
    );
    assert!(!msgs.is_empty());
    let zero_redelegate = msgs.iter().find(|msg| {
        if let CosmosMsg::Staking(StakingMsg::Redelegate {
            src_validator: _,
            dst_validator: _,
            amount,
        }) = msg
        {
            return amount.amount.is_zero();
        }
        false
    });
    // Amount cannot be zero
    assert!(zero_redelegate.is_none());

    let surplus_validators = Vec::from([
        ValidatorDelegation {
            address: "a".to_string(),
            delegation_diff: DelegationDiff::Surplus,
            diff_amount: Uint128::new(5000),
        },
        ValidatorDelegation {
            address: "b".to_string(),
            delegation_diff: DelegationDiff::Surplus,
            diff_amount: Uint128::new(5000),
        },
    ]);
    let deficient_validators = vec![
        ValidatorDelegation {
            address: "c".to_string(),
            delegation_diff: DelegationDiff::Deficit,
            diff_amount: Uint128::new(7500),
        },
        ValidatorDelegation {
            address: "d".to_string(),
            delegation_diff: DelegationDiff::Deficit,
            diff_amount: Uint128::new(2500),
        },
    ];
    let msgs = get_restaking_msgs(
        delegator.clone(),
        surplus_validators,
        deficient_validators,
        "denom".to_string(),
    );
    let mut net_amounts = msgs
        .into_iter()
        .map(|msg| {
            if let CosmosMsg::Staking(StakingMsg::Redelegate {
                dst_validator,
                src_validator: _,
                amount,
            }) = msg
            {
                return (dst_validator, amount.amount.u128());
            }
            panic!("bad cosmos msg");
        })
        .fold(HashMap::new(), |mut h, pair| {
            h.entry(pair.0)
                .and_modify(|amount| *amount += pair.1)
                .or_insert(pair.1);
            h
        })
        .into_iter()
        .collect::<Vec<_>>();
    net_amounts.sort_by_key(|a: &(String, u128)| a.1);
    // Should redelegate in totality
    assert_eq!(
        net_amounts,
        vec![("d".to_string(), 2500_u128), ("c".to_string(), 7500_u128)]
    );
}

#[test]
fn test_get_delegate_to_validator_msgs() {
    let delegator = "delegator".to_string();
    let delegate_amount = Uint128::new(50000);
    let coin_denom = "denom".to_string();
    let validators = vec![
        Validator {
            address: "abc".to_string(),
            weight: 10,
        },
        Validator {
            address: "bcd".to_string(),
            weight: 20,
        },
    ];
    let msgs =
        get_delegate_to_validator_msgs(delegator.clone(), delegate_amount, coin_denom, validators);
    let amounts = msgs
        .into_iter()
        .map(|msg| {
            if let CosmosMsg::Any(AnyMsg { type_url: _, value }) = msg {
                let proto::babylon::epoching::v1::MsgWrappedUndelegate { msg } =
                    proto::babylon::epoching::v1::MsgWrappedUndelegate::decode(value.as_slice())
                        .unwrap();
                let proto::cosmos::staking::v1beta1::MsgUndelegate {
                    delegator_address,
                    validator_address,
                    amount,
                } = msg.unwrap();
                assert!(delegator_address == delegator, "bad delegator");
                return (
                    validator_address,
                    Uint128::from_str(&amount.unwrap().amount).unwrap(),
                );
            }
            panic!("bad cosmos msg");
        })
        .fold(HashMap::new(), |mut h, pair| {
            h.entry(pair.0)
                .and_modify(|amount| *amount += pair.1)
                .or_insert(pair.1);
            h
        });
    assert_approx_eq!(
        amounts.get("abc").unwrap() * Uint128::new(2),
        *amounts.get("bcd").unwrap(),
        "0.001"
    );
    assert_eq!(
        amounts.iter().map(|a| a.1).sum::<Uint128>(),
        delegate_amount
    );
}

#[test]
fn test_get_delegate_to_validator_msgs_should_skip_zero_delegate_amount() {
    let delegator = "delegator".to_string();
    let validators = Vec::from([
        Validator {
            address: "a".to_string(),
            weight: 0,
        },
        Validator {
            address: "b".to_string(),
            weight: 9,
        },
        Validator {
            address: "c".to_string(),
            weight: 1,
        },
    ]);
    let msgs = get_delegate_to_validator_msgs(
        delegator.clone(),
        Uint128::from(100_u128),
        "denom".to_string(),
        validators,
    );

    let zero_amount_msg = msgs.iter().find(|msg| {
        if let CosmosMsg::Staking(StakingMsg::Delegate {
            validator: _,
            amount,
        }) = msg
        {
            if amount.amount == Uint128::zero() {
                return true;
            }
        }
        false
    });
    assert!(zero_amount_msg.is_none());
}

#[test]
fn test_get_validator_delegation_map_base_on_weight_should_delegate_remaining_amount() {
    let validators = Vec::from([
        Validator {
            address: "a".to_string(),
            weight: 1,
        },
        Validator {
            address: "b".to_string(),
            weight: 100,
        },
        Validator {
            address: "c".to_string(),
            weight: 1000,
        },
    ]);
    let total_delegated_amount = Uint128::from(500_000_u128);

    assert_eq!(
        get_validator_delegation_map_base_on_weight(validators, total_delegated_amount)
            .values()
            .sum::<Uint128>(),
        total_delegated_amount
    );
}

#[test]
fn test_get_actual_total_delegated() {
    let mut querier = MockQuerier::default();
    let delegator_addr = Addr::unchecked("delegator");
    let denom = "denom".to_string();
    let validator_addr_a = "a".to_string();
    let validator_addr_b = "b".to_string();
    let validator_addr_c = "c".to_string();
    let validators = &[
        cosmwasm_std::Validator::create(
            validator_addr_a.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            validator_addr_b.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            validator_addr_c.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
    ];
    let delegations = &[
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            validator_addr_a.clone(),
            Coin::new(Uint128::new(1000), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            validator_addr_b.clone(),
            Coin::new(Uint128::new(2000), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            validator_addr_c.clone(),
            Coin::new(Uint128::new(500), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
    ];
    querier
        .staking
        .update(denom.clone(), validators, delegations);
    let querier_wrapper = QuerierWrapper::<Empty>::new(&querier);

    let total_delegated = get_actual_total_delegated(
        querier_wrapper,
        delegator_addr.to_string(),
        denom.clone(),
        Vec::from([validator_addr_a, validator_addr_b]),
    )
    .unwrap();
    assert_eq!(total_delegated, Uint128::new(3000));
}

#[test]
fn test_get_unclaimed_reward() {
    let mut querier = MockQuerier::default();
    let delegator_addr = Addr::unchecked("delegator");
    let denom = "denom".to_string();
    let validator_addr_a = "a".to_string();
    let validator_addr_b = "b".to_string();
    let validator_addr_c = "c".to_string();

    querier.distribution.set_rewards(
        validator_addr_a.clone(),
        delegator_addr.clone(),
        Vec::from([DecCoin::new(
            Decimal256::from_str("2000.0").unwrap(),
            denom.clone(),
        )]),
    );
    querier.distribution.set_rewards(
        validator_addr_b.clone(),
        delegator_addr.clone(),
        Vec::from([DecCoin::new(
            Decimal256::from_str("1000.0").unwrap(),
            denom.clone(),
        )]),
    );
    querier.distribution.set_rewards(
        validator_addr_c.clone(),
        delegator_addr.clone(),
        Vec::from([DecCoin::new(
            Decimal256::from_str("500.0").unwrap(),
            denom.clone(),
        )]),
    );

    let querier_wrapper = QuerierWrapper::<Empty>::new(&querier);

    let unclaimed_rewards = get_unclaimed_reward(
        querier_wrapper,
        delegator_addr.to_string(),
        denom.clone(),
        Vec::from([validator_addr_a.clone(), validator_addr_b.clone()]),
    )
    .unwrap();

    assert_eq!(unclaimed_rewards, Uint128::new(3000));
}

#[test]
fn test_get_mock_total_reward() {
    assert_eq!(
        get_mock_total_reward(Uint128::new(1000)),
        Uint128::new(1005)
    );
}

#[test]
fn test_get_validator_delegation_map_with_total_bond() {
    let mut deps = mock_dependencies();
    let delegator_addr = Addr::unchecked("delegator");
    let validator_addr_a = "a".to_string();
    let validator_addr_b = "b".to_string();
    let validator_addr_other = "other".to_string();
    let denom = "denom".to_string();
    let mut querier = MockQuerier::default();

    let validators_cosm = &[
        cosmwasm_std::Validator::create(
            validator_addr_a.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            validator_addr_b.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
    ];
    let validators = Vec::from([
        Validator {
            weight: 10,
            address: validator_addr_a.clone(),
        },
        Validator {
            weight: 20,
            address: validator_addr_b.clone(),
        },
        Validator {
            weight: 20,
            address: validator_addr_other.clone(),
        },
    ]);
    let delegations = &[
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            validator_addr_a.clone(),
            Coin::new(Uint128::new(1000), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            validator_addr_b.clone(),
            Coin::new(Uint128::new(2000), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
    ];
    querier
        .staking
        .update(denom.clone(), validators_cosm, delegations);

    deps.querier = querier;

    let (validator_delegation_map, total_delegated_amount) =
        get_validator_delegation_map_with_total_bond(
            deps.as_ref(),
            delegator_addr.to_string(),
            validators,
        )
        .unwrap();

    assert_eq!(total_delegated_amount, Uint128::new(3000));
    assert_eq!(
        validator_delegation_map.get(&validator_addr_a).unwrap(),
        Uint128::new(1000)
    );
    assert_eq!(
        validator_delegation_map.get(&validator_addr_b).unwrap(),
        Uint128::new(2000)
    );
    assert_eq!(
        validator_delegation_map.get(&validator_addr_other).unwrap(),
        Uint128::zero()
    );
}

#[test]
fn test_adjust_validators_delegation() {
    let mut deps = mock_dependencies();
    let delegator_addr = Addr::unchecked("delegator");
    let validator_addr_a = "a".to_string();
    let validator_addr_b = "b".to_string();
    let validator_addr_c = "c".to_string();
    let denom = "denom".to_string();
    let mut querier = MockQuerier::default();

    let validators_cosm = &[
        cosmwasm_std::Validator::create(
            validator_addr_a.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            validator_addr_b.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
    ];
    let prev_validators = Vec::from([
        Validator {
            weight: 50,
            address: validator_addr_b.clone(),
        },
        Validator {
            weight: 10,
            address: validator_addr_c.clone(),
        },
    ]);
    let validators = Vec::from([
        Validator {
            weight: 10,
            address: validator_addr_a.clone(),
        },
        Validator {
            weight: 20,
            address: validator_addr_b.clone(),
        },
    ]);
    let delegations = &[
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            validator_addr_a.clone(),
            Coin::new(Uint128::new(1000), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            validator_addr_b.clone(),
            Coin::new(Uint128::new(2000), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
    ];

    querier
        .staking
        .update(denom.clone(), validators_cosm, delegations);
    deps.querier = querier;

    let mut parameters = mock_parameters();
    parameters.underlying_coin_denom = denom.clone();

    PARAMETERS.save(deps.as_mut().storage, &parameters).unwrap();

    let (validator_delegation_map, total_delegated_amount) =
        get_validator_delegation_map_with_total_bond(
            deps.as_ref(),
            delegator_addr.to_string(),
            prev_validators.clone(),
        )
        .unwrap();

    let correct_validator_delegation_map =
        get_validator_delegation_map_base_on_weight(validators.clone(), total_delegated_amount);

    let (surplus_validators, deficient_validators) =
        get_surplus_deficit_validators(validator_delegation_map, correct_validator_delegation_map);

    let msgs: Vec<CosmosMsg> = get_restaking_msgs(
        delegator_addr.to_string(),
        surplus_validators,
        deficient_validators,
        denom,
    );

    assert!(!msgs.is_empty());

    assert_eq!(
        msgs,
        adjust_validators_delegation(
            deps.as_mut(),
            delegator_addr.clone(),
            prev_validators.clone(),
            validators.clone()
        )
        .unwrap()
    );
}

#[test]
fn test_get_undelegate_msgs() {
    let undelegate_amount = Uint128::new(1000);
    let delegator = "delegator".to_string();
    let coin_denom = "denom".to_string();
    let validator_a = "a".to_string();
    let validator_b = "b".to_string();
    let validator_c = "c".to_string();
    let other_validator = "other".to_string();
    let validator_delegation_ratio = HashMap::from([
        (validator_a.clone(), Decimal::from_str("0.4").unwrap()),
        (validator_b.clone(), Decimal::from_str("0.3").unwrap()),
        (validator_c.clone(), Decimal::from_str("0.2").unwrap()),
        (other_validator.clone(), Decimal::from_str("0.0").unwrap()),
    ]);
    let (total_undelegate_amount, msgs, mut atts) = get_undelegate_msgs(
        delegator.clone(),
        undelegate_amount,
        coin_denom.clone(),
        validator_delegation_ratio,
    );
    assert_eq!(total_undelegate_amount, Uint128::new(900));

    assert_eq!(msgs.len(), 3);
    let mut undelegates = msgs
        .into_iter()
        .map(|msg| {
            let CosmosMsg::Any(AnyMsg { type_url: _, value }) = msg else {
                panic!("wrong cosmos msg");
            };
            let proto::babylon::epoching::v1::MsgWrappedDelegate { msg } =
                proto::babylon::epoching::v1::MsgWrappedDelegate::decode(value.as_slice()).unwrap();
            let proto::cosmos::staking::v1beta1::MsgDelegate {
                delegator_address,
                validator_address,
                amount,
            } = msg.unwrap();
            let amount = amount.unwrap();
            assert!(delegator_address == delegator, "bad delegator");
            assert!(amount.denom == coin_denom, "wrong denom");
            (
                validator_address,
                Uint128::from_str(&amount.amount).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    undelegates.sort_by_key(|undelegate| undelegate.0.clone());
    assert_eq!(
        undelegates,
        Vec::from([
            (validator_a.clone(), Uint128::new(400)),
            (validator_b.clone(), Uint128::new(300)),
            (validator_c.clone(), Uint128::new(200)),
        ])
    );
    atts.sort_by_key(|att| att.key.clone());
    assert_eq!(
        atts,
        Vec::from([
            Attribute {
                key: validator_a.clone(),
                value: "400".to_string()
            },
            Attribute {
                key: validator_b.clone(),
                value: "300".to_string()
            },
            Attribute {
                key: validator_c.clone(),
                value: "200".to_string()
            },
        ])
    );
}

#[test]
fn test_unstake_request_in_batch() {
    let mut deps = mock_dependencies();
    let mut env = mock_env();
    env.block.height = 500_000;
    let sender = "sender".to_string();
    let staker = "staker".to_string();
    let unstake_amount = Uint128::new(10000);
    let pending_batch_id = 10;
    let token_count = 5;
    let channel_id = 1;
    let pending_batch = Batch {
        id: pending_batch_id,
        total_liquid_stake: Uint128::new(100),
        expected_native_unstaked: None,
        received_native_unstaked: None,
        unbond_records_count: 0,
        next_batch_action_time: None,
        status: BatchStatus::Pending,
    };
    batches()
        .save(deps.as_mut().storage, pending_batch_id, &pending_batch)
        .unwrap();
    TOKEN_COUNT
        .save(deps.as_mut().storage, &token_count)
        .unwrap();
    PENDING_BATCH_ID
        .save(deps.as_mut().storage, &pending_batch_id)
        .unwrap();
    let params = mock_parameters();
    PARAMETERS.save(deps.as_mut().storage, &params).unwrap();
    WITHDRAW_REWARD_QUEUE
        .save(deps.as_mut().storage, &vec![])
        .unwrap();
    REWARD_BALANCE
        .save(deps.as_mut().storage, &Uint128::zero())
        .unwrap();
    SUPPLY_QUEUE
        .save(
            deps.as_mut().storage,
            &SupplyQueue {
                mint: vec![],
                burn: vec![],
                epoch_period: 360,
            },
        )
        .unwrap();

    let unstake_request_event = unstake_request_in_batch(
        env.clone(),
        deps.as_mut().storage,
        sender.clone(),
        staker.clone(),
        unstake_amount,
        Some(channel_id),
        None,
        None,
        None,
    )
    .unwrap();

    let new_token_count = token_count + 1;
    assert_eq!(
        TOKEN_COUNT.load(deps.as_mut().storage).unwrap(),
        new_token_count
    );
    assert_eq!(
        batches()
            .load(deps.as_mut().storage, pending_batch_id)
            .unwrap(),
        Batch {
            total_liquid_stake: pending_batch.total_liquid_stake + unstake_amount,
            unbond_records_count: pending_batch.unbond_records_count + 1,
            ..pending_batch
        }
    );
    assert_eq!(
        unbond_record()
            .load(deps.as_mut().storage, new_token_count)
            .unwrap(),
        UnbondRecord {
            id: new_token_count,
            batch_id: pending_batch_id,
            height: env.block.height,
            channel_id: Some(channel_id),
            sender: sender.clone(),
            staker: staker.clone(),
            amount: unstake_amount,
            released_height: 0,
            released: false,
            recipient: None,
            recipient_channel_id: None,
        }
    );
    assert_eq!(unstake_request_event.ty, UNSTAKE_REQUEST_EVENT);
    let sender_attr = &unstake_request_event
        .attributes
        .iter()
        .find(|a| a.key == "sender")
        .unwrap()
        .value;
    assert_eq!(sender_attr, &sender);
    let staker_attr = &unstake_request_event
        .attributes
        .iter()
        .find(|a| a.key == "staker")
        .unwrap()
        .value;
    assert_eq!(staker_attr, &staker);
    let channel_id_attr = &unstake_request_event
        .attributes
        .iter()
        .find(|a| a.key == "channel_id")
        .unwrap()
        .value;
    assert_eq!(channel_id_attr, &channel_id.to_string());
    let unbond_amount_attr = &unstake_request_event
        .attributes
        .iter()
        .find(|a| a.key == "unbond_amount")
        .unwrap()
        .value;
    assert_eq!(unbond_amount_attr, &unstake_amount.to_string());
    let time_attr = &unstake_request_event
        .attributes
        .iter()
        .find(|a| a.key == "time")
        .unwrap()
        .value;
    assert_eq!(time_attr, &env.block.time.nanos().to_string());
    let record_id_attr = &unstake_request_event
        .attributes
        .iter()
        .find(|a| a.key == "record_id")
        .unwrap()
        .value;
    assert_eq!(record_id_attr, &new_token_count.to_string());
}

#[test]
fn test_process_bond() {
    let app = App::default();
    let api = app.api();
    let mut deps = mock_dependencies();
    let querier = MockQuerier::default();
    let sender = "sender".to_string();
    let staker = "staker".to_string();
    let delegator = api.addr_make("delegator");
    let amount = Uint128::new(10000);
    let bond_time = 36000;
    let block_height = 10_000_000;
    let salt = "0x0000000000000000000000000000000000000000000000000000000000000001".to_string();
    let params = mock_parameters();

    let validator_addr_a = "a".to_string();
    let validator_addr_b = "b".to_string();
    let channel_id = Some(10);
    let validators = Vec::from([
        Validator {
            weight: 10,
            address: validator_addr_a.clone(),
        },
        Validator {
            weight: 20,
            address: validator_addr_b.clone(),
        },
    ]);
    let validators_reg = ValidatorsRegistry {
        validators: validators.clone(),
    };
    let querier_wrapper = QuerierWrapper::<Empty>::new(&querier);
    let state = State {
        exchange_rate: Decimal::from_str("1.1").unwrap(),
        total_bond_amount: Uint128::new(20_000),
        total_delegated_amount: Uint128::new(15_000),
        total_supply: Uint128::new(10_000),
        bond_counter: 5,
        last_bond_time: 50000,
    };
    STATE.save(deps.as_mut().storage, &state).unwrap();
    let mut supply_queue = SupplyQueue {
        mint: Vec::from([
            MintQueue {
                amount: Uint128::new(100),
                block: 1,
            },
            MintQueue {
                amount: Uint128::new(200),
                block: 2,
            },
        ]),
        burn: Vec::from([
            BurnQueue {
                amount: Uint128::new(50),
                block: 2,
            },
            BurnQueue {
                amount: Uint128::new(70),
                block: 3,
            },
        ]),
        epoch_period: 10,
    };
    SUPPLY_QUEUE
        .save(deps.as_mut().storage, &supply_queue)
        .unwrap();

    let (msgs, sub_msgs, bond_data) = process_bond(
        deps.as_mut().storage,
        querier_wrapper,
        sender,
        staker,
        delegator.clone(),
        amount,
        bond_time,
        params.clone(),
        validators_reg,
        salt,
        channel_id,
        block_height,
        None,
        None,
        false,
        None,
    )
    .unwrap();

    let updated_state = STATE.load(deps.as_mut().storage).unwrap();
    let total_bond_amount = get_mock_total_reward(state.total_bond_amount);

    let mint_amount = calc::calculate_staking_token_from_rate(amount, updated_state.exchange_rate);

    normalize_supply_queue(&mut supply_queue, block_height);
    let normalized_total_supply =
        normalize_total_supply(state.total_supply, &supply_queue.mint, &supply_queue.burn);
    assert_eq!(
        updated_state.exchange_rate,
        Decimal::from_ratio(total_bond_amount, normalized_total_supply),
    );

    assert_eq!(updated_state.bond_counter, state.bond_counter + 1);
    assert_eq!(updated_state.total_supply, state.total_supply + mint_amount);
    assert_eq!(updated_state.total_bond_amount, total_bond_amount + amount);
    assert_eq!(updated_state.total_supply, state.total_supply + mint_amount);
    assert_eq!(
        updated_state.total_delegated_amount,
        state.total_delegated_amount + amount,
    );
    assert_eq!(
        updated_state.total_delegated_amount,
        state.total_delegated_amount + amount,
    );
    assert_eq!(updated_state.last_bond_time, bond_time);

    assert!(sub_msgs.is_empty());
    assert_eq!(
        bond_data,
        BondData {
            mint_amount,
            delegated_amount: updated_state.total_delegated_amount,
            total_bond_amount: updated_state.total_bond_amount,
            exchange_rate: Decimal::from_ratio(total_bond_amount, state.total_supply),
            total_supply: updated_state.total_supply,
            reward_balance: Uint128::zero(),
            unclaimed_reward: Uint128::zero(),
        }
    );
    assert_eq!(
        msgs,
        get_delegate_to_validator_msgs(
            delegator.to_string(),
            amount,
            params.underlying_coin_denom,
            validators
        )
    );
}

#[test]
fn test_delegate() {
    let mut deps = mock_dependencies();
    let env: cosmwasm_std::Env = mock_env();
    let delegator = env.contract.address.clone();
    let amount = Uint128::new(10000);
    let params = mock_parameters();

    PARAMETERS.save(deps.as_mut().storage, &params).unwrap();

    let validator_addr_a = "a".to_string();
    let validator_addr_b = "b".to_string();
    let validators: Vec<Validator> = Vec::from([
        Validator {
            weight: 50,
            address: validator_addr_a.clone(),
        },
        Validator {
            weight: 50,
            address: validator_addr_b.clone(),
        },
    ]);

    let total_delegation = Uint128::new(7_466_305_228);
    deps = setup_validators_delegation(
        deps,
        &delegator,
        validators.as_slice(),
        NATIVE_DENOM.to_string(),
        total_delegation,
    );

    crate::state::VALIDATORS_REGISTRY
        .save(
            deps.as_mut().storage,
            &ValidatorsRegistry {
                validators: validators.clone(),
            },
        )
        .unwrap();

    let exchange_rate = Decimal::from_str("1.9787253322").unwrap();
    let state = State {
        exchange_rate,
        total_bond_amount: total_delegation,
        total_delegated_amount: total_delegation,
        total_supply: Uint128::new(3_773_290_364),
        bond_counter: 5,
        last_bond_time: 50000,
    };
    STATE.save(deps.as_mut().storage, &state).unwrap();

    println!("state: {state:?}");

    WITHDRAW_REWARD_QUEUE
        .save(deps.as_mut().storage, &vec![])
        .unwrap();

    REWARD_BALANCE
        .save(deps.as_mut().storage, &Uint128::zero())
        .unwrap();

    let supply_queue: SupplyQueue = SupplyQueue {
        mint: Vec::from([
            MintQueue {
                amount: Uint128::new(100),
                block: 1,
            },
            MintQueue {
                amount: Uint128::new(200),
                block: 2,
            },
        ]),
        burn: Vec::from([
            BurnQueue {
                amount: Uint128::new(50),
                block: 2,
            },
            BurnQueue {
                amount: Uint128::new(70),
                block: 3,
            },
        ]),
        epoch_period: 10,
    };
    SUPPLY_QUEUE
        .save(deps.as_mut().storage, &supply_queue)
        .unwrap();

    let min_mint_amount = (Decimal::from_ratio(amount, Uint128::one()) / exchange_rate)
        .to_uint_floor()
        .strict_sub(Uint128::new(50u128));

    println!("min_mint_amount: {min_mint_amount}");

    let querier: QuerierWrapper<Empty> = QuerierWrapper::new(&deps.querier);

    let (msgs, new_bond_data) =
        delegate(&mut deps.storage, querier, env, amount, min_mint_amount).unwrap();

    let updated_state = STATE.load(deps.as_mut().storage).unwrap();

    println!("new_bond_data: {new_bond_data:?}");
    println!("updated_state: {updated_state:?}");

    assert_eq!(updated_state.bond_counter, state.bond_counter + 1);
    assert_eq!(
        updated_state.total_supply,
        state.total_supply + new_bond_data.mint_amount
    );
    assert_eq!(
        updated_state.total_bond_amount,
        state.total_bond_amount + new_bond_data.bond_amount
    );
    assert_eq!(
        updated_state.total_supply,
        state.total_supply + new_bond_data.mint_amount
    );
    assert_eq!(
        updated_state.total_delegated_amount,
        state.total_delegated_amount + amount,
    );

    assert_eq!(
        msgs,
        get_delegate_to_validator_msgs(
            delegator.to_string(),
            amount,
            params.underlying_coin_denom,
            validators
        )
    );
}

#[test]
fn test_remote_bond_from_invalid_address() {
    let app = App::default();
    let api = app.api();
    let mut deps = mock_dependencies();
    let env: cosmwasm_std::Env = mock_env();

    let sender = "sender".into_addr();

    let sender_addr = api.addr_make(sender.as_str());
    let params = mock_parameters();

    PARAMETERS.save(deps.as_mut().storage, &params).unwrap();

    STATUS
        .save(
            deps.as_mut().storage,
            &crate::state::Status {
                bond_is_paused: false,
                unbond_is_paused: false,
            },
        )
        .unwrap();

    let min_mint_amount = Uint128::new(100u128);

    let info = cosmwasm_std::testing::message_info(
        &sender_addr,
        &cosmwasm_std::coins(10000, NATIVE_DENOM),
    );

    let user: Addr = Addr::unchecked("user");

    let res = remote_bond(
        deps.as_mut(),
        env.clone(),
        info,
        min_mint_amount,
        user.clone(),
    );

    assert!(res.is_err());
}

#[test]
fn test_submit_pending_batch() {
    let mut deps = mock_dependencies();
    let time = Timestamp::from_seconds(1_000_000);
    let block_height = 10000;
    let sender = deps.api.addr_make("sender");
    let delegator = deps.api.addr_make("delegator");
    let pending_batch_id = 10;
    let mut pending_batch = Batch {
        id: pending_batch_id,
        total_liquid_stake: Uint128::new(100),
        expected_native_unstaked: None,
        received_native_unstaked: None,
        unbond_records_count: 0,
        next_batch_action_time: None,
        status: BatchStatus::Pending,
    };
    let params = mock_parameters();
    let validator_addr_a = "a".to_string();
    let validator_addr_b = "b".to_string();
    let validators = Vec::from([
        Validator {
            weight: 10,
            address: validator_addr_a.clone(),
        },
        Validator {
            weight: 20,
            address: validator_addr_b.clone(),
        },
    ]);
    let validators_cosm = &[
        cosmwasm_std::Validator::create(
            validator_addr_a.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            validator_addr_b.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
    ];
    let delegations = &[
        cosmwasm_std::FullDelegation::create(
            delegator.clone(),
            validator_addr_a.clone(),
            Coin::new(Uint128::new(1000), params.underlying_coin_denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
        cosmwasm_std::FullDelegation::create(
            delegator.clone(),
            validator_addr_b.clone(),
            Coin::new(Uint128::new(2000), params.underlying_coin_denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
    ];
    let mut querier = MockQuerier::default();
    querier.staking.update(
        params.underlying_coin_denom.clone(),
        validators_cosm,
        delegations,
    );
    querier.distribution.set_rewards(
        validator_addr_a.clone(),
        delegator.clone(),
        Vec::from([DecCoin::new(
            Decimal256::from_str("20.0").unwrap(),
            params.underlying_coin_denom.clone(),
        )]),
    );
    querier.distribution.set_rewards(
        validator_addr_b.clone(),
        delegator.clone(),
        Vec::from([DecCoin::new(
            Decimal256::from_str("10.0").unwrap(),
            params.underlying_coin_denom.clone(),
        )]),
    );
    deps.querier = querier;
    let validators_reg = ValidatorsRegistry {
        validators: validators.clone(),
    };
    let state = State {
        exchange_rate: Decimal::from_str("1.1").unwrap(),
        total_bond_amount: Uint128::new(20_000),
        total_delegated_amount: Uint128::new(15_000),
        total_supply: Uint128::new(100_000),
        bond_counter: 5,
        last_bond_time: 50000,
    };
    STATE.save(deps.as_mut().storage, &state).unwrap();
    let supply_queue = SupplyQueue {
        mint: Vec::from([
            MintQueue {
                amount: Uint128::new(100),
                block: 1,
            },
            MintQueue {
                amount: Uint128::new(200),
                block: 2,
            },
        ]),
        burn: Vec::from([
            BurnQueue {
                amount: Uint128::new(50),
                block: 2,
            },
            BurnQueue {
                amount: Uint128::new(70),
                block: 3,
            },
        ]),
        epoch_period: 10,
    };
    SUPPLY_QUEUE
        .save(deps.as_mut().storage, &supply_queue)
        .unwrap();
    let reward_balance = Uint128::new(100_000);
    REWARD_BALANCE
        .save(deps.as_mut().storage, &reward_balance)
        .unwrap();

    WITHDRAW_REWARD_QUEUE
        .save(deps.as_mut().storage, &vec![])
        .unwrap();

    let (msgs, events) = submit_pending_batch(
        deps.as_mut(),
        block_height,
        time,
        sender,
        delegator.clone(),
        &mut pending_batch,
        params.clone(),
        validators_reg.clone(),
    )
    .unwrap();

    assert_eq!(
        PENDING_BATCH_ID.load(&deps.storage).unwrap(),
        pending_batch_id + 1
    );

    let queue: Vec<crate::state::WithdrawRewardQueue> = vec![WithdrawRewardQueue {
        amount: Uint128::new(30u128),
        block: 10000,
    }];
    assert_eq!(queue, WITHDRAW_REWARD_QUEUE.load(&deps.storage).unwrap());

    let updated_batch = batches().load(&deps.storage, pending_batch.id).unwrap();
    assert!(matches!(updated_batch.status, BatchStatus::Submitted));
    assert!(updated_batch.next_batch_action_time.is_some());
    assert!(msgs.iter().all(|msg| {
        match msg {
            CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            }) => {
                // Check burn msg
                if *contract_addr != params.cw20_address.to_string() || !funds.is_empty() {
                    return false;
                }
                let msg: cw20::Cw20ExecuteMsg = from_json(msg).unwrap();
                let cw20::Cw20ExecuteMsg::Burn { amount } = msg else {
                    return false;
                };
                amount == pending_batch.total_liquid_stake
            }
            CosmosMsg::Any(AnyMsg { type_url: _, value }) => {
                let proto::babylon::epoching::v1::MsgWrappedUndelegate { msg } =
                    proto::babylon::epoching::v1::MsgWrappedUndelegate::decode(value.as_slice())
                        .unwrap();
                let proto::cosmos::staking::v1beta1::MsgUndelegate {
                    delegator_address,
                    validator_address,
                    amount,
                } = msg.unwrap();
                let amount = amount.unwrap();
                validators
                    .iter()
                    .map(|v| v.address.clone())
                    .collect::<Vec<_>>()
                    .contains(&validator_address)
                    && delegator_address == delegator.to_string()
                    && amount.denom == params.underlying_coin_denom
                    && !Uint128::from_str(&amount.amount).unwrap().is_zero()
            }
            _ => false,
        }
    }));
    assert!(events
        .iter()
        .all(|event| event.ty == SUBMIT_BATCH_EVENT || event.ty == UNBOND_EVENT));
}

#[test]
fn validator_restaking_adjustment() {
    use std::collections::HashMap;

    let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();
    let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    validator_delegation_map.insert("A".into(), Uint128::new(50000));
    validator_delegation_map.insert("B".into(), Uint128::new(50000));

    correct_validator_delegation_map.insert("B".into(), Uint128::new(30000));
    correct_validator_delegation_map.insert("C".into(), Uint128::new(30000));
    correct_validator_delegation_map.insert("D".into(), Uint128::new(40000));

    let (surplus, deficit) = crate::utils::delegation::get_surplus_deficit_validators(
        validator_delegation_map,
        correct_validator_delegation_map,
    );

    let denom = "muno".to_string();
    let delegator = "bbn123glhewf3w66cquy6hr7urjv3589srheqj3abc".to_string();
    let msgs =
        crate::utils::delegation::get_restaking_msgs(delegator, surplus, deficit, denom.clone());

    let staking_msg = get_redelegate_msg(30000, denom.clone(), "A".to_string(), "C".to_string());
    assert_eq!(msgs.first().unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(20000, denom.clone(), "A".to_string(), "D".to_string());
    assert_eq!(msgs.get(1).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(20000, denom.clone(), "B".to_string(), "D".to_string());
    assert_eq!(msgs.get(2).unwrap(), &staking_msg);

    println!("msgs: {msgs:#?}");
}

#[test]
fn validator_restaking_adjustment_2() {
    use std::collections::HashMap;

    let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();
    let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    validator_delegation_map.insert("A".into(), Uint128::new(50000));
    validator_delegation_map.insert("B".into(), Uint128::new(50000));

    correct_validator_delegation_map.insert("A".into(), Uint128::new(20000));
    correct_validator_delegation_map.insert("B".into(), Uint128::new(15000));
    correct_validator_delegation_map.insert("C".into(), Uint128::new(35000));
    correct_validator_delegation_map.insert("D".into(), Uint128::new(30000));

    let (surplus, deficit) = crate::utils::delegation::get_surplus_deficit_validators(
        validator_delegation_map,
        correct_validator_delegation_map,
    );

    let denom = "muno".to_string();
    let delegator = "bbn123glhewf3w66cquy6hr7urjv3589srheqj3abc".to_string();
    let msgs =
        crate::utils::delegation::get_restaking_msgs(delegator, surplus, deficit, denom.clone());

    let staking_msg = get_redelegate_msg(30000, denom.clone(), "A".to_string(), "C".to_string());
    assert_eq!(msgs.first().unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(5000, denom.clone(), "B".to_string(), "C".to_string());
    assert_eq!(msgs.get(1).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(30000, denom.clone(), "B".to_string(), "D".to_string());
    assert_eq!(msgs.get(2).unwrap(), &staking_msg);

    println!("msgs: {msgs:#?}");
}

#[test]
fn validator_restaking_adjustment_3() {
    use std::collections::HashMap;

    let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();
    let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    validator_delegation_map.insert("A".into(), Uint128::new(30000));
    validator_delegation_map.insert("B".into(), Uint128::new(40000));
    validator_delegation_map.insert("C".into(), Uint128::new(30000));

    correct_validator_delegation_map.insert("B".into(), Uint128::new(25000));
    correct_validator_delegation_map.insert("C".into(), Uint128::new(25000));
    correct_validator_delegation_map.insert("D".into(), Uint128::new(50000));

    let (surplus, deficit) = crate::utils::delegation::get_surplus_deficit_validators(
        validator_delegation_map,
        correct_validator_delegation_map,
    );

    let denom = "muno".to_string();
    let delegator = "bbn123glhewf3w66cquy6hr7urjv3589srheqj3abc".to_string();
    let msgs =
        crate::utils::delegation::get_restaking_msgs(delegator, surplus, deficit, denom.clone());

    let staking_msg = get_redelegate_msg(30000, denom.clone(), "A".to_string(), "D".to_string());
    assert_eq!(msgs.first().unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(15000, denom.clone(), "B".to_string(), "D".to_string());
    assert_eq!(msgs.get(1).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(5000, denom.clone(), "C".to_string(), "D".to_string());
    assert_eq!(msgs.get(2).unwrap(), &staking_msg);

    println!("\nmsgs: {msgs:#?}");
}

#[test]
fn validator_restaking_adjustment_4() {
    use std::collections::HashMap;

    let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();
    let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    validator_delegation_map.insert("A".into(), Uint128::new(23000));
    validator_delegation_map.insert("B".into(), Uint128::new(77000));

    correct_validator_delegation_map.insert("A".into(), Uint128::new(20000));
    correct_validator_delegation_map.insert("B".into(), Uint128::new(15000));
    correct_validator_delegation_map.insert("C".into(), Uint128::new(35000));
    correct_validator_delegation_map.insert("D".into(), Uint128::new(12000));
    correct_validator_delegation_map.insert("E".into(), Uint128::new(18000));

    let (surplus, deficit) = crate::utils::delegation::get_surplus_deficit_validators(
        validator_delegation_map,
        correct_validator_delegation_map,
    );

    let denom = "muno".to_string();
    let delegator = "bbn123glhewf3w66cquy6hr7urjv3589srheqj3abc".to_string();
    let msgs =
        crate::utils::delegation::get_restaking_msgs(delegator, surplus, deficit, denom.clone());

    let staking_msg = get_redelegate_msg(3000, denom.clone(), "A".to_string(), "C".to_string());
    assert_eq!(msgs.first().unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(32000, denom.clone(), "B".to_string(), "C".to_string());
    assert_eq!(msgs.get(1).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(12000, denom.clone(), "B".to_string(), "D".to_string());
    assert_eq!(msgs.get(2).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(18000, denom.clone(), "B".to_string(), "E".to_string());
    assert_eq!(msgs.get(3).unwrap(), &staking_msg);

    println!("\nmsgs: {msgs:#?}");
}

#[test]
fn validators_restaking_adjustment_5() {
    let validator_delegation_map = HashMap::from([
        (
            "bbnvaloper140l6y2gp3gxvay6qtn70re7z2s0gn57zx9gg4e".to_string(),
            Uint128::new(2600),
        ),
        (
            "bbnvaloper1eunu7l7qfmemdw4xv7apejl28jzgd3t346dh63".to_string(),
            Uint128::new(700),
        ),
        (
            "bbnvaloper1symf474wnypes2d3mecllqk6l26rwz8mx605rm".to_string(),
            Uint128::new(2600),
        ),
        (
            "bbnvaloper1fyfnvvswqjmg2xlpx2grldmlnuzqj6zj2hc8hd".to_string(),
            Uint128::new(3100),
        ),
        (
            "bbnvaloper1004nqmppj9tvwf0l5gawl747lg452vl35m5x0x".to_string(),
            Uint128::new(500),
        ),
        (
            "bbnvaloper1g2dslw8hn62tj3yyjcw3t7gx7lxghna7auh4qw".to_string(),
            Uint128::new(500),
        ),
    ]);

    let validators = Vec::from([
        Validator {
            address: "bbnvaloper140l6y2gp3gxvay6qtn70re7z2s0gn57zx9gg4e".to_string(),
            weight: 17,
        },
        Validator {
            address: "bbnvaloper1eunu7l7qfmemdw4xv7apejl28jzgd3t346dh63".to_string(),
            weight: 2,
        },
        Validator {
            address: "bbnvaloper1symf474wnypes2d3mecllqk6l26rwz8mx605rm".to_string(),
            weight: 17,
        },
        Validator {
            address: "bbnvaloper1fyfnvvswqjmg2xlpx2grldmlnuzqj6zj2hc8hd".to_string(),
            weight: 20,
        },
        Validator {
            address: "bbnvaloper1004nqmppj9tvwf0l5gawl747lg452vl35m5x0x".to_string(),
            weight: 12,
        },
        Validator {
            address: "bbnvaloper1g2dslw8hn62tj3yyjcw3t7gx7lxghna7auh4qw".to_string(),
            weight: 8,
        },
        Validator {
            address: "bbnvaloper163zszfeemrqfyg3jlasztzmy0eea8l8qjlvlz2".to_string(),
            weight: 12,
        },
        Validator {
            address: "bbnvaloper1l5c6cf6rps3vq65hmk73hqv2epj6wrn2vlkawa".to_string(),
            weight: 12,
        },
    ]);

    let total_delegated_amount = Uint128::new(10000u128);

    let correct_validator_delegation_map =
        get_validator_delegation_map_base_on_weight(validators, total_delegated_amount);

    println!("correct_validator_delegation_map : {correct_validator_delegation_map:#?}");

    let (surplus_validators, deficient_validators) =
        get_surplus_deficit_validators(validator_delegation_map, correct_validator_delegation_map);

    let denom = "ubbn".to_string();
    let delegator = "bbn123glhewf3w66cquy6hr7urjv3589srheqj3abc".to_string();
    let msgs: Vec<CosmosMsg> = get_restaking_msgs(
        delegator,
        surplus_validators,
        deficient_validators,
        denom.clone(),
    );
    println!("\nmsgs: {msgs:#?}");

    let staking_msg = get_redelegate_msg(
        700u128,
        denom.clone(),
        "bbnvaloper140l6y2gp3gxvay6qtn70re7z2s0gn57zx9gg4e".to_string(),
        "bbnvaloper1004nqmppj9tvwf0l5gawl747lg452vl35m5x0x".to_string(),
    );
    assert_eq!(msgs.first().unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(
        200u128,
        denom.clone(),
        "bbnvaloper140l6y2gp3gxvay6qtn70re7z2s0gn57zx9gg4e".to_string(),
        "bbnvaloper163zszfeemrqfyg3jlasztzmy0eea8l8qjlvlz2".to_string(),
    );
    assert_eq!(msgs.get(1).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(
        500u128,
        denom.clone(),
        "bbnvaloper1eunu7l7qfmemdw4xv7apejl28jzgd3t346dh63".to_string(),
        "bbnvaloper163zszfeemrqfyg3jlasztzmy0eea8l8qjlvlz2".to_string(),
    );
    assert_eq!(msgs.get(2).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(
        500u128,
        denom.clone(),
        "bbnvaloper1fyfnvvswqjmg2xlpx2grldmlnuzqj6zj2hc8hd".to_string(),
        "bbnvaloper163zszfeemrqfyg3jlasztzmy0eea8l8qjlvlz2".to_string(),
    );
    assert_eq!(msgs.get(3).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(
        300u128,
        denom.clone(),
        "bbnvaloper1fyfnvvswqjmg2xlpx2grldmlnuzqj6zj2hc8hd".to_string(),
        "bbnvaloper1g2dslw8hn62tj3yyjcw3t7gx7lxghna7auh4qw".to_string(),
    );
    assert_eq!(msgs.get(4).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(
        300u128,
        denom.clone(),
        "bbnvaloper1fyfnvvswqjmg2xlpx2grldmlnuzqj6zj2hc8hd".to_string(),
        "bbnvaloper1l5c6cf6rps3vq65hmk73hqv2epj6wrn2vlkawa".to_string(),
    );
    assert_eq!(msgs.get(5).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(
        900u128,
        denom.clone(),
        "bbnvaloper1symf474wnypes2d3mecllqk6l26rwz8mx605rm".to_string(),
        "bbnvaloper1l5c6cf6rps3vq65hmk73hqv2epj6wrn2vlkawa".to_string(),
    );
    assert_eq!(msgs.get(6).unwrap(), &staking_msg);
}

#[test]
fn validators_restaking_adjustment_6() {
    let lavender_5 = "bbnvaloper140l6y2gp3gxvay6qtn70re7z2s0gn57zx9gg4e".to_string();
    let crypto_crew = "bbnvaloper1eunu7l7qfmemdw4xv7apejl28jzgd3t346dh63".to_string();
    let block_hunters = "bbnvaloper1symf474wnypes2d3mecllqk6l26rwz8mx605rm".to_string();
    let node_01 = "bbnvaloper1fyfnvvswqjmg2xlpx2grldmlnuzqj6zj2hc8hd".to_string();
    let figment = "bbnvaloper1004nqmppj9tvwf0l5gawl747lg452vl35m5x0x".to_string();
    let fiona: String = "bbnvaloper1g2dslw8hn62tj3yyjcw3t7gx7lxghna7auh4qw".to_string();
    let cosmos_spaces = "bbnvaloper163zszfeemrqfyg3jlasztzmy0eea8l8qjlvlz2".to_string();
    let everstake = "bbnvaloper1l5c6cf6rps3vq65hmk73hqv2epj6wrn2vlkawa".to_string();

    let validator_delegation_map = HashMap::from([
        (cosmos_spaces.clone(), Uint128::new(2_565_228)),
        (node_01.clone(), Uint128::new(2_147_588)),
        (lavender_5.clone(), Uint128::new(1_825_450)),
        (block_hunters.clone(), Uint128::new(1_825_450)),
        (figment.clone(), Uint128::new(1_288_553)),
        (fiona.clone(), Uint128::new(859_035)),
        (crypto_crew.clone(), Uint128::new(214_758)),
        (everstake.clone(), Uint128::new(11341)),
    ]);

    let validators = Vec::from([
        Validator {
            address: cosmos_spaces.clone(),
            weight: 12,
        },
        Validator {
            address: node_01.clone(),
            weight: 18,
        },
        Validator {
            address: lavender_5.clone(),
            weight: 16,
        },
        Validator {
            address: block_hunters.clone(),
            weight: 16,
        },
        Validator {
            address: figment.clone(),
            weight: 12,
        },
        Validator {
            address: fiona.clone(),
            weight: 12,
        },
        Validator {
            address: crypto_crew.clone(),
            weight: 2,
        },
        Validator {
            address: everstake.clone(),
            weight: 12,
        },
    ]);

    let total_delegated_amount = validator_delegation_map.values().copied().sum();

    let correct_validator_delegation_map =
        get_validator_delegation_map_base_on_weight(validators, total_delegated_amount);

    println!("correct_validator_delegation_map : {correct_validator_delegation_map:#?}");

    let (surplus_validators, deficient_validators) =
        get_surplus_deficit_validators(validator_delegation_map, correct_validator_delegation_map);

    let denom = "ubbn".to_string();

    let delegator = "bbn123glhewf3w66cquy6hr7urjv3589srheqj3abc".to_string();
    let msgs: Vec<CosmosMsg> = get_restaking_msgs(
        delegator,
        surplus_validators,
        deficient_validators,
        denom.clone(),
    );
    println!("\nmsgs: {msgs:#?}");

    let staking_msg = get_redelegate_msg(65u128, denom.clone(), figment, fiona.clone());
    assert_eq!(msgs.first().unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(107_466_u128, denom.clone(), lavender_5, fiona.clone());
    assert_eq!(msgs.get(1).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(
        321_922_u128,
        denom.clone(),
        cosmos_spaces.clone(),
        fiona.clone(),
    );
    assert_eq!(msgs.get(2).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(
        954_815_u128,
        denom.clone(),
        cosmos_spaces,
        everstake.clone(),
    );
    assert_eq!(msgs.get(3).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(10u128, denom.clone(), crypto_crew, everstake.clone());
    assert_eq!(msgs.get(4).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(214_856_u128, denom.clone(), node_01, everstake.clone());
    assert_eq!(msgs.get(5).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(107_466_u128, denom.clone(), block_hunters, everstake);
    assert_eq!(msgs.get(6).unwrap(), &staking_msg);
}

fn get_redelegate_msg(
    amount: u128,
    denom: String,
    src_validator: String,
    dst_validator: String,
) -> CosmosMsg {
    CosmosMsg::Staking(StakingMsg::Redelegate {
        amount: Coin {
            denom,
            amount: Uint128::new(amount),
        },
        src_validator,
        dst_validator,
    })
}

#[test]
fn validators_restaking_adjustment_7() {
    let mut deps = mock_dependencies();
    let delegator_addr = Addr::unchecked("delegator");
    let blockhunters = "blockhunters".to_string();
    let node01 = "01node".to_string();
    let everstake = "everstake".to_string();
    let figment = "figment".to_string();
    let cosmosspaces = "cosmosspaces".to_string();
    let cryptocrew = "cryptocrew".to_string();

    let denom = "denom".to_string();
    let mut querier = MockQuerier::default();

    let validators_cosm = &[
        cosmwasm_std::Validator::create(
            blockhunters.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            node01.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            everstake.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            figment.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            cosmosspaces.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            cryptocrew.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
    ];
    let delegations = &[
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            blockhunters.clone(),
            Coin::new(Uint128::new(716), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            node01.clone(),
            Coin::new(Uint128::new(713), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
    ];

    let prev_validators = Vec::from([
        Validator {
            weight: 50,
            address: blockhunters.clone(),
        },
        Validator {
            weight: 50,
            address: node01.clone(),
        },
    ]);
    let validators = Vec::from([
        Validator {
            weight: 21,
            address: blockhunters.clone(),
        },
        Validator {
            weight: 23,
            address: node01.clone(),
        },
        Validator {
            weight: 17,
            address: everstake.clone(),
        },
        Validator {
            weight: 19,
            address: figment.clone(),
        },
        Validator {
            weight: 18,
            address: cosmosspaces.clone(),
        },
        Validator {
            weight: 2,
            address: cryptocrew.clone(),
        },
    ]);

    querier
        .staking
        .update(denom.clone(), validators_cosm, delegations);
    deps.querier = querier;

    let mut parameters = mock_parameters();
    parameters.underlying_coin_denom = denom.clone();

    PARAMETERS.save(deps.as_mut().storage, &parameters).unwrap();

    let (validator_delegation_map, total_delegated_amount) =
        get_validator_delegation_map_with_total_bond(
            deps.as_ref(),
            delegator_addr.to_string(),
            prev_validators.clone(),
        )
        .unwrap();

    println!(
        "validator_delegation_map: {:#?}",
        validator_delegation_map.clone()
    );
    println!("total_delegated_amount: {total_delegated_amount:#?}");

    let correct_validator_delegation_map =
        get_validator_delegation_map_base_on_weight(validators.clone(), total_delegated_amount);

    println!("correct_validator_delegation_map: {correct_validator_delegation_map:#?}");

    let (surplus_validators, deficient_validators) = get_surplus_deficit_validators(
        validator_delegation_map.clone(),
        correct_validator_delegation_map.clone(),
    );

    let msgs: Vec<CosmosMsg> = get_restaking_msgs(
        delegator_addr.to_string(),
        surplus_validators,
        deficient_validators,
        denom.clone(),
    );

    let mut new_delegation_map: HashMap<String, Uint128> = validator_delegation_map.clone();

    for msg in &msgs {
        if let CosmosMsg::Staking(StakingMsg::Redelegate {
            amount,
            src_validator,
            dst_validator,
            ..
        }) = msg
        {
            let amount = amount.amount;
            let src_validator = src_validator.clone();
            let dst_validator = dst_validator.clone();

            new_delegation_map
                .entry(src_validator.clone())
                .and_modify(|v| *v -= amount);

            new_delegation_map
                .entry(dst_validator.clone())
                .or_insert(Uint128::zero());

            new_delegation_map
                .entry(dst_validator.clone())
                .and_modify(|v| *v += amount);
        }
    }

    assert_eq!(
        new_delegation_map.clone(),
        correct_validator_delegation_map.clone()
    );
}

#[test]
fn validators_restaking_adjustment_8() {
    let mut deps = mock_dependencies();
    let delegator_addr = Addr::unchecked("delegator");
    let blockhunters = "blockhunters".to_string();
    let node01 = "01node".to_string();
    let everstake = "everstake".to_string();
    let figment = "figment".to_string();
    let cosmosspaces = "cosmosspaces".to_string();
    let cryptocrew = "cryptocrew".to_string();

    let denom = "denom".to_string();
    let mut querier = MockQuerier::default();

    let validators_cosm = &[
        cosmwasm_std::Validator::create(
            blockhunters.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            node01.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            everstake.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            figment.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            cosmosspaces.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
        cosmwasm_std::Validator::create(
            cryptocrew.clone(),
            Decimal::default(),
            Decimal::default(),
            Decimal::default(),
        ),
    ];
    let delegations = &[
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            figment.clone(),
            Coin::new(Uint128::new(258_586), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            blockhunters.clone(),
            Coin::new(Uint128::new(315_935), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            cosmosspaces.clone(),
            Coin::new(Uint128::new(56967), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            everstake.clone(),
            Coin::new(Uint128::new(215_445), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            node01.clone(),
            Coin::new(Uint128::new(445_739), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
        cosmwasm_std::FullDelegation::create(
            delegator_addr.clone(),
            cryptocrew.clone(),
            Coin::new(Uint128::new(143_630), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
    ];

    let prev_validators = Vec::from([
        Validator {
            weight: 21,
            address: blockhunters.clone(),
        },
        Validator {
            weight: 23,
            address: node01.clone(),
        },
        Validator {
            weight: 17,
            address: everstake.clone(),
        },
        Validator {
            weight: 19,
            address: figment.clone(),
        },
        Validator {
            weight: 18,
            address: cosmosspaces.clone(),
        },
        Validator {
            weight: 2,
            address: cryptocrew.clone(),
        },
    ]);
    let validators = Vec::from([
        Validator {
            weight: 25,
            address: blockhunters.clone(),
        },
        Validator {
            weight: 25,
            address: node01.clone(),
        },
        Validator {
            weight: 10,
            address: everstake.clone(),
        },
        Validator {
            weight: 18,
            address: figment.clone(),
        },
        Validator {
            weight: 17,
            address: cosmosspaces.clone(),
        },
        Validator {
            weight: 5,
            address: cryptocrew.clone(),
        },
    ]);

    querier
        .staking
        .update(denom.clone(), validators_cosm, delegations);
    deps.querier = querier;

    let mut parameters = mock_parameters();
    parameters.underlying_coin_denom = denom.clone();

    PARAMETERS.save(deps.as_mut().storage, &parameters).unwrap();

    let (validator_delegation_map, total_delegated_amount) =
        get_validator_delegation_map_with_total_bond(
            deps.as_ref(),
            delegator_addr.to_string(),
            prev_validators.clone(),
        )
        .unwrap();

    println!(
        "validator_delegation_map: {:#?}",
        validator_delegation_map.clone()
    );
    println!("total_delegated_amount: {total_delegated_amount:#?}");

    let correct_validator_delegation_map =
        get_validator_delegation_map_base_on_weight(validators.clone(), total_delegated_amount);

    println!("correct_validator_delegation_map: {correct_validator_delegation_map:#?}");

    let (surplus_validators, deficient_validators) = get_surplus_deficit_validators(
        validator_delegation_map.clone(),
        correct_validator_delegation_map.clone(),
    );

    let msgs: Vec<CosmosMsg> = get_restaking_msgs(
        delegator_addr.to_string(),
        surplus_validators,
        deficient_validators,
        denom.clone(),
    );
    println!("msgs: {msgs:#?}");

    let mut new_delegation_map: HashMap<String, Uint128> = validator_delegation_map.clone();

    for msg in &msgs {
        if let CosmosMsg::Staking(StakingMsg::Redelegate {
            amount,
            src_validator,
            dst_validator,
        }) = msg
        {
            let amount = amount.amount;
            let src_validator = src_validator.clone();
            let dst_validator = dst_validator.clone();

            new_delegation_map
                .entry(src_validator.clone())
                .and_modify(|v| *v -= amount);

            new_delegation_map
                .entry(dst_validator.clone())
                .or_insert(Uint128::zero());

            new_delegation_map
                .entry(dst_validator.clone())
                .and_modify(|v| *v += amount);
        }
    }

    println!("new_delegation_map: {new_delegation_map:#?}");

    assert_eq!(
        new_delegation_map.clone(),
        correct_validator_delegation_map.clone()
    );
}
