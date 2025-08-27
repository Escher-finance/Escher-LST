use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{
    Addr, Attribute, Coin, CosmosMsg, DecCoin, Decimal, Decimal256, Empty, QuerierWrapper,
    StakingMsg, Timestamp, Uint128, assert_approx_eq, from_json,
    testing::{MockQuerier, mock_dependencies, mock_env},
};
use cw_multi_test::App;

use crate::{
    event::{SUBMIT_BATCH_EVENT, UNBOND_EVENT, UNSTAKE_REQUEST_EVENT},
    msg::{BondData, DelegationDiff, ValidatorDelegation},
    state::{
        PARAMETERS, PENDING_BATCH_ID, REWARD_BALANCE, STATE, State, TOKEN_COUNT, UnbondRecord,
        VALIDATORS_REGISTRY, Validator, ValidatorsRegistry, unbond_record,
    },
    tests::mock_parameters,
    utils::{
        batch::{Batch, BatchStatus, batches},
        calc,
        delegation::{
            adjust_validators_delegation, get_actual_total_delegated,
            get_delegate_to_validator_msgs, get_mock_total_reward, get_restaking_msgs,
            get_surplus_deficit_validators, get_unbond_all_messages, get_unclaimed_reward,
            get_undelegate_msgs, get_validator_delegation_map_base_on_weight,
            get_validator_delegation_map_with_total_bond, process_bond, submit_pending_batch,
            unstake_request_in_batch,
        },
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
    let total_delegated_amount = Uint128::new(250000);
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
        surplus_validators,
        deficient_validators,
        "denom".to_string(),
    );
    let mut net_amounts = msgs
        .into_iter()
        .filter_map(|msg| {
            if let CosmosMsg::Staking(StakingMsg::Redelegate {
                src_validator: _,
                dst_validator,
                amount,
            }) = msg
            {
                return Some((dst_validator, amount.amount.u128()));
            }
            None
        })
        .fold(HashMap::new(), |mut h, pair| {
            h.entry(pair.0)
                .and_modify(|amount| *amount += pair.1)
                .or_insert(pair.1);
            h
        })
        .into_iter()
        .collect::<Vec<_>>();
    net_amounts.sort_by_key(|a| a.1);
    // Should redelegate in totality
    assert_eq!(
        net_amounts,
        vec![("d".to_string(), 2500_u128), ("c".to_string(), 7500_u128)]
    );
}

#[test]
fn test_get_delegate_to_validator_msgs() {
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
    let msgs = get_delegate_to_validator_msgs(delegate_amount, coin_denom, validators);
    let amounts = msgs
        .into_iter()
        .filter_map(|msg| {
            if let CosmosMsg::Staking(StakingMsg::Delegate { validator, amount }) = msg {
                return Some((validator, amount.amount));
            }
            None
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
    let msgs =
        get_delegate_to_validator_msgs(Uint128::from(100_u128), "denom".to_string(), validators);

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
    let total_delegated_amount = Uint128::from(500000_u128);

    assert_eq!(
        get_validator_delegation_map_base_on_weight(validators, total_delegated_amount)
            .values()
            .sum::<Uint128>(),
        total_delegated_amount
    )
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
fn test_get_unbond_all_messages() {
    let mut deps = mock_dependencies();
    let delegator_addr = Addr::unchecked("delegator");
    let validator_addr_a = "a".to_string();
    let validator_addr_b = "b".to_string();
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
    ]);
    let delegations = &[
        cosmwasm_std::FullDelegation::create(
            Addr::unchecked("other_delegator_addr"),
            validator_addr_a.clone(),
            Coin::new(Uint128::new(500), denom.clone()),
            Coin::default(),
            Vec::default(),
        ),
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

    let deps_mut = deps.as_mut();

    let mut parameters = mock_parameters();
    parameters.underlying_coin_denom = denom.clone();
    PARAMETERS.save(deps_mut.storage, &parameters).unwrap();
    VALIDATORS_REGISTRY
        .save(deps_mut.storage, &ValidatorsRegistry { validators })
        .unwrap();

    let msgs = get_unbond_all_messages(deps_mut, delegator_addr.clone()).unwrap();
    assert_eq!(msgs.len(), 2);

    let mut undelegates = msgs
        .into_iter()
        .map(|msg| {
            let CosmosMsg::Staking(StakingMsg::Undelegate { validator, amount }) = msg else {
                panic!()
            };
            if amount.denom != denom {
                panic!()
            }
            (validator, amount.amount)
        })
        .collect::<Vec<_>>();
    undelegates.sort_by_key(|x| x.0.clone());
    assert_eq!(undelegates[0], (validator_addr_a, Uint128::new(1000)));
    assert_eq!(undelegates[1], (validator_addr_b, Uint128::new(2000)));
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

    let msgs: Vec<CosmosMsg> = get_restaking_msgs(surplus_validators, deficient_validators, denom);

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
    let coin_denom = "denom".to_string();
    let validator_a = "a".to_string();
    let validator_b = "b".to_string();
    let validator_c = "c".to_string();
    let other_validator = "other".to_string();
    let validator_delegation_ratio = HashMap::from([
        (validator_a.to_string(), Decimal::from_str("0.4").unwrap()),
        (validator_b.to_string(), Decimal::from_str("0.3").unwrap()),
        (validator_c.to_string(), Decimal::from_str("0.2").unwrap()),
        (
            other_validator.to_string(),
            Decimal::from_str("0.0").unwrap(),
        ),
    ]);
    let (total_undelegate_amount, msgs, mut atts) = get_undelegate_msgs(
        undelegate_amount,
        coin_denom.clone(),
        validator_delegation_ratio,
    );
    assert_eq!(total_undelegate_amount, Uint128::new(00));

    assert_eq!(msgs.len(), 3);
    let mut undelegates = msgs
        .into_iter()
        .map(|msg| {
            let CosmosMsg::Staking(StakingMsg::Undelegate { validator, amount }) = msg else {
                panic!("wrong cosmos msg");
            };
            if amount.denom != coin_denom {
                panic!("wrong denom");
            }
            (validator, amount.amount)
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
    env.block.height = 500000;
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
    REWARD_BALANCE
        .save(deps.as_mut().storage, &Uint128::zero())
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
    let block_height = 10000000;
    let delegator = api.addr_make("delegator");
    let amount = Uint128::new(10000);
    let bond_time = 36000;
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

    let (msgs, sub_msgs, bond_data) = process_bond(
        deps.as_mut().storage,
        querier_wrapper,
        sender,
        staker,
        delegator,
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
    )
    .unwrap();

    let updated_state = STATE.load(deps.as_mut().storage).unwrap();
    let total_bond_amount = get_mock_total_reward(state.total_bond_amount);

    let mint_amount = calc::calculate_staking_token_from_rate(amount, updated_state.exchange_rate);

    assert_eq!(
        updated_state.exchange_rate,
        Decimal::from_ratio(total_bond_amount + amount, state.total_supply + mint_amount),
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
            amount,
            params.underlying_coin_denom.to_string(),
            validators
        )
    );
}

#[test]
fn test_submit_pending_batch() {
    let mut deps = mock_dependencies();
    let time = Timestamp::from_seconds(1000000);
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
    let reward_balance = Uint128::new(100_000);
    REWARD_BALANCE
        .save(deps.as_mut().storage, &reward_balance)
        .unwrap();

    let (msgs, events) = submit_pending_batch(
        deps.as_mut(),
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
    let querier_wrapper = QuerierWrapper::<Empty>::new(&deps.querier);
    let total_reward = get_unclaimed_reward(
        querier_wrapper,
        delegator.to_string(),
        params.underlying_coin_denom.clone(),
        validators_reg
            .validators
            .iter()
            .map(|v| v.address.clone())
            .collect(),
    )
    .unwrap();
    assert!(!total_reward.is_zero());
    assert_eq!(
        REWARD_BALANCE.load(&deps.storage).unwrap(),
        reward_balance + total_reward
    );
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
            CosmosMsg::Staking(StakingMsg::Undelegate { validator, amount }) => {
                validators
                    .iter()
                    .map(|v| v.address.clone())
                    .collect::<Vec<_>>()
                    .contains(validator)
                    && amount.denom == params.underlying_coin_denom
                    && !amount.amount.is_zero()
            }
            _ => false,
        }
    }));
    assert!(
        events
            .iter()
            .all(|event| event.ty == SUBMIT_BATCH_EVENT || event.ty == UNBOND_EVENT)
    );
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
    let msgs = crate::utils::delegation::get_restaking_msgs(surplus, deficit, denom.clone());

    let staking_msg = get_redelegate_msg(30000, denom.clone(), "A".to_string(), "C".to_string());
    assert_eq!(msgs.first().unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(20000, denom.clone(), "A".to_string(), "D".to_string());
    assert_eq!(msgs.get(1).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(20000, denom.clone(), "B".to_string(), "D".to_string());
    assert_eq!(msgs.get(2).unwrap(), &staking_msg);

    println!("msgs: {:#?}", msgs);
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
    let msgs = crate::utils::delegation::get_restaking_msgs(surplus, deficit, denom.clone());

    let staking_msg = get_redelegate_msg(30000, denom.clone(), "A".to_string(), "C".to_string());
    assert_eq!(msgs.first().unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(5000, denom.clone(), "B".to_string(), "C".to_string());
    assert_eq!(msgs.get(1).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(30000, denom.clone(), "B".to_string(), "D".to_string());
    assert_eq!(msgs.get(2).unwrap(), &staking_msg);

    println!("msgs: {:#?}", msgs);
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
    let msgs = crate::utils::delegation::get_restaking_msgs(surplus, deficit, denom.clone());

    let staking_msg = get_redelegate_msg(30000, denom.clone(), "A".to_string(), "D".to_string());
    assert_eq!(msgs.first().unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(15000, denom.clone(), "B".to_string(), "D".to_string());
    assert_eq!(msgs.get(1).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(5000, denom.clone(), "C".to_string(), "D".to_string());
    assert_eq!(msgs.get(2).unwrap(), &staking_msg);

    println!("\nmsgs: {:#?}", msgs);
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
    let msgs = crate::utils::delegation::get_restaking_msgs(surplus, deficit, denom.clone());

    let staking_msg = get_redelegate_msg(3000, denom.clone(), "A".to_string(), "C".to_string());
    assert_eq!(msgs.first().unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(32000, denom.clone(), "B".to_string(), "C".to_string());
    assert_eq!(msgs.get(1).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(12000, denom.clone(), "B".to_string(), "D".to_string());
    assert_eq!(msgs.get(2).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(18000, denom.clone(), "B".to_string(), "E".to_string());
    assert_eq!(msgs.get(3).unwrap(), &staking_msg);

    println!("\nmsgs: {:#?}", msgs);
}

#[test]
fn validators_restaking_adjustment_5() {
    let validator_delegation_map = HashMap::from([
        (
            "bbnvaloper140l6y2gp3gxvay6qtn70re7z2s0gn57zxgg4e".to_string(),
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
            "bbnvaloper1004nqmppjtvwf0l5gawl747lg452vl35m5x0x".to_string(),
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

    println!(
        "correct_validator_delegation_map : {:#?}",
        correct_validator_delegation_map
    );

    let (surplus_validators, deficient_validators) =
        get_surplus_deficit_validators(validator_delegation_map, correct_validator_delegation_map);

    let denom = "ubbn".to_string();

    let msgs: Vec<CosmosMsg> =
        get_restaking_msgs(surplus_validators, deficient_validators, denom.clone());
    println!("\nmsgs: {:#?}", msgs);

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
        (cosmos_spaces.clone(), Uint128::new(2565228)),
        (node_01.clone(), Uint128::new(2147588)),
        (lavender_5.clone(), Uint128::new(1825450)),
        (block_hunters.clone(), Uint128::new(1825450)),
        (figment.clone(), Uint128::new(1288553)),
        (fiona.clone(), Uint128::new(859035)),
        (crypto_crew.clone(), Uint128::new(214758)),
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

    let total_delegated_amount = Uint128::new(10737403u128);

    let correct_validator_delegation_map =
        get_validator_delegation_map_base_on_weight(validators, total_delegated_amount);

    println!(
        "correct_validator_delegation_map : {:#?}",
        correct_validator_delegation_map
    );

    let (surplus_validators, deficient_validators) =
        get_surplus_deficit_validators(validator_delegation_map, correct_validator_delegation_map);

    let denom = "ubbn".to_string();

    let msgs: Vec<CosmosMsg> =
        get_restaking_msgs(surplus_validators, deficient_validators, denom.clone());

    println!("\nmsgs: {:#?}", msgs);

    let staking_msg = get_redelegate_msg(65u128, denom.clone(), figment, fiona.clone());
    assert_eq!(msgs.first().unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(107466u128, denom.clone(), lavender_5, fiona.clone());
    assert_eq!(msgs.get(1).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(
        321922u128,
        denom.clone(),
        cosmos_spaces.clone(),
        fiona.clone(),
    );
    assert_eq!(msgs.get(2).unwrap(), &staking_msg);

    let staking_msg =
        get_redelegate_msg(954815u128, denom.clone(), cosmos_spaces, everstake.clone());
    assert_eq!(msgs.get(3).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(10u128, denom.clone(), crypto_crew, everstake.clone());
    assert_eq!(msgs.get(4).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(214856u128, denom.clone(), node_01, everstake.clone());
    assert_eq!(msgs.get(5).unwrap(), &staking_msg);

    let staking_msg = get_redelegate_msg(107466u128, denom.clone(), block_hunters, everstake);
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
