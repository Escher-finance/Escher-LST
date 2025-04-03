use crate::{
    msg::{DelegationDiff, ValidatorDelegation},
    state::{Validator, ValidatorsRegistry, PARAMETERS, VALIDATORS_REGISTRY},
    tests::mock_parameters,
    utils::delegation::*,
};
use cosmwasm_std::{
    assert_approx_eq,
    testing::{mock_dependencies, MockQuerier},
    Addr, Coin, CosmosMsg, DecCoin, Decimal, Decimal256, Empty, QuerierWrapper, StakingMsg,
    Uint128,
};
use std::{collections::HashMap, str::FromStr};

#[test]
fn test_get_undelegate_from_validator_msgs() {
    let undelegate_amount = Uint128::from(500642u32);
    let coin_denom = "muno".to_string();

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
    let (total_undelegate_amount, undelegate_msgs, _) =
        get_undelegate_from_validator_msgs(undelegate_amount, coin_denom.clone(), validators);
    let undelegate_msgs_unwrapped = undelegate_msgs
        .iter()
        .filter_map(|msg| {
            if let CosmosMsg::Staking(StakingMsg::Undelegate { validator, amount }) = msg {
                return Some((validator, amount.amount));
            }
            return None;
        })
        .collect::<Vec<_>>();
    assert_eq!(undelegate_msgs_unwrapped.len(), 2);
    assert_eq!(
        undelegate_msgs_unwrapped[0].1 + undelegate_msgs_unwrapped[1].1,
        total_undelegate_amount
    );
    assert_approx_eq!(undelegate_amount, total_undelegate_amount, "0.001");
    assert_approx_eq!(
        undelegate_msgs_unwrapped[0].1 * Uint128::from(2_u128),
        undelegate_msgs_unwrapped[1].1,
        "0.001"
    );
}

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
    assert!(msgs.len() > 0);
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
            .iter()
            .map(|(_addr, amount)| amount)
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
