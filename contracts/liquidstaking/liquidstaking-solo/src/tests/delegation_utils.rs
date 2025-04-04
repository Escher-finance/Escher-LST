use crate::{
    event::{SUBMIT_BATCH_EVENT, UNBOND_EVENT, UNSTAKE_REQUEST_EVENT},
    msg::{BondData, DelegationDiff, Ucs03ExecuteMsg, ValidatorDelegation},
    proto,
    state::{
        unbond_record, BurnQueue, MintQueue, QuoteToken, State, SupplyQueue, UnbondRecord,
        Validator, ValidatorsRegistry, PARAMETERS, PENDING_BATCH_ID, QUOTE_TOKEN, REWARD_BALANCE,
        STATE, SUPPLY_QUEUE, TOKEN_COUNT,
    },
    tests::mock_parameters,
    utils::{
        batch::{batches, Batch, BatchStatus},
        calc::{self, normalize_supply_queue, normalize_total_supply},
        delegation::*,
    },
};
use cosmwasm_std::{
    assert_approx_eq, from_json,
    testing::{mock_dependencies, mock_env, MockQuerier},
    Addr, AnyMsg, Attribute, Coin, CosmosMsg, DecCoin, Decimal, Decimal256, Empty, QuerierWrapper,
    StakingMsg, Timestamp, Uint128, Uint256,
};
use cw_multi_test::App;
use prost::Message;
use std::{collections::HashMap, str::FromStr};
use unionlabs_primitives::{encoding::HexPrefixed, Bytes, H256};

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
        delegator.clone(),
        surplus_validators,
        deficient_validators,
        "denom".to_string(),
    );
    let mut net_amounts = msgs
        .into_iter()
        .map(|msg| {
            if let CosmosMsg::Any(AnyMsg { type_url: _, value }) = msg {
                let proto::babylon::epoching::v1::MsgWrappedBeginRedelegate { msg } =
                    proto::babylon::epoching::v1::MsgWrappedBeginRedelegate::decode(
                        value.as_slice(),
                    )
                    .unwrap();
                let proto::cosmos::staking::v1beta1::MsgBeginRedelegate {
                    delegator_address,
                    validator_src_address: _,
                    validator_dst_address,
                    amount,
                } = msg.unwrap();
                if delegator_address != delegator {
                    panic!("bad delegator");
                }
                return (
                    validator_dst_address,
                    u128::from_str(&amount.unwrap().amount).unwrap(),
                );
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
    net_amounts.sort_by_key(|a| a.1);
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
                if delegator_address != delegator {
                    panic!("bad delegator");
                }
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
        (validator_a.to_string(), Decimal::from_str("0.4").unwrap()),
        (validator_b.to_string(), Decimal::from_str("0.3").unwrap()),
        (validator_c.to_string(), Decimal::from_str("0.2").unwrap()),
        (
            other_validator.to_string(),
            Decimal::from_str("0.0").unwrap(),
        ),
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
            if delegator_address != delegator {
                panic!("bad delegator");
            }
            if amount.denom != coin_denom {
                panic!("wrong denom");
            }
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
fn test_get_transfer_token_cosmos_msg() {
    let mut deps = mock_dependencies();
    let quote_token = QuoteToken {
        channel_id: 1,
        quote_token: "0xbeef".to_string(),
        lst_quote_token: "lst_quote_token".to_string(),
    };
    QUOTE_TOKEN
        .save(deps.as_mut().storage, quote_token.channel_id, &quote_token)
        .unwrap();
    let staker = "0xffff".to_string();
    let channel_id = Some(quote_token.channel_id);
    let time = Timestamp::default();
    let ucs03_relay_contract = "ucs03_relay".to_string();
    let undelegate_amount = Uint128::new(1000);
    let denom = "denom".to_string();
    let salt = "0x0000000000000000000000000000000000000000000000000000000000000001".to_string();

    let amount_funds = Vec::from([Coin {
        denom: denom.clone(),
        amount: undelegate_amount,
    }]);

    // channel_id is None
    let CosmosMsg::Bank(cosmwasm_std::BankMsg::Send { to_address, amount }) =
        get_transfer_token_cosmos_msg(
            deps.as_mut().storage,
            staker.clone(),
            None,
            time,
            ucs03_relay_contract.clone(),
            undelegate_amount,
            denom.clone(),
            salt.clone(),
        )
        .unwrap()
    else {
        panic!("expected bank send msg");
    };
    assert_eq!(to_address, staker.clone());
    assert_eq!(amount, amount_funds.clone());

    // channel_id is Some
    let CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
        contract_addr,
        msg,
        funds,
    }) = get_transfer_token_cosmos_msg(
        deps.as_mut().storage,
        staker.clone(),
        channel_id,
        time,
        ucs03_relay_contract.clone(),
        undelegate_amount,
        denom.clone(),
        salt.clone(),
    )
    .unwrap()
    else {
        panic!("expected wasm execute msg");
    };
    assert_eq!(contract_addr, ucs03_relay_contract);
    assert_eq!(funds, amount_funds);
    let ucs03_execute_msg: Ucs03ExecuteMsg = from_json(msg).unwrap();
    let Ucs03ExecuteMsg::Transfer {
        channel_id: ucs03_channel_id,
        receiver: ucs03_receiver,
        base_token: ucs03_base_token,
        base_amount: ucs03_base_amount,
        quote_token: ucs03_quote_token,
        quote_amount: ucs03_quote_amount,
        timeout_height: ucs03_timeout_height,
        timeout_timestamp: ucs03_timeout_timestamp,
        salt: ucs03_salt,
    } = ucs03_execute_msg;
    assert_eq!(ucs03_channel_id, channel_id.unwrap());
    assert_eq!(
        ucs03_receiver,
        Bytes::<HexPrefixed>::from_str(staker.as_str()).unwrap()
    );
    assert_eq!(ucs03_base_token, denom.clone());
    assert_eq!(ucs03_base_amount, undelegate_amount);
    assert_eq!(
        ucs03_quote_token,
        Bytes::<HexPrefixed>::from_str(quote_token.quote_token.as_str()).unwrap()
    );
    assert_eq!(ucs03_quote_amount, Uint256::from(undelegate_amount));
    assert_eq!(ucs03_timeout_height, 0);
    assert_eq!(
        ucs03_timeout_timestamp,
        time.plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET).nanos()
    );
    assert_eq!(
        ucs03_salt,
        H256::<HexPrefixed>::from_str(salt.as_str()).unwrap(),
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
    let channel_id = Some(1);
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

    let unstake_request_event = unstake_request_in_batch(
        env.clone(),
        deps.as_mut().storage,
        sender.clone(),
        staker.clone(),
        unstake_amount,
        channel_id,
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
            channel_id,
            sender: sender.clone(),
            staker: staker.clone(),
            amount: unstake_amount,
            released_height: 0,
            released: false
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
    assert_eq!(channel_id_attr, &channel_id.unwrap().to_string());
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
    let block_height = 10000000;
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
        validators: Vec::from(validators.clone()),
    };
    let querier_wrapper = QuerierWrapper::<Empty>::new(&querier);
    let state = State {
        exchange_rate: Decimal::from_str("1.1").unwrap(),
        total_bond_amount: Uint128::new(20_000),
        total_delegated_amount: Uint128::new(15_000),
        total_supply: Uint128::new(100_000),
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
            total_supply: updated_state.total_supply
        }
    );
    assert_eq!(
        msgs,
        get_delegate_to_validator_msgs(
            delegator.to_string(),
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
        validators: Vec::from(validators.clone()),
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
                return amount == pending_batch.total_liquid_stake;
            }
            CosmosMsg::Staking(StakingMsg::Undelegate { validator, amount }) => {
                return validators
                    .iter()
                    .map(|v| v.address.clone())
                    .collect::<Vec<_>>()
                    .contains(validator)
                    && amount.denom == params.underlying_coin_denom
                    && !amount.amount.is_zero();
            }
            _ => false,
        }
    }));
    assert!(events
        .iter()
        .all(|event| event.ty == SUBMIT_BATCH_EVENT || event.ty == UNBOND_EVENT));
}
