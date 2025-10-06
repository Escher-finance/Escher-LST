use std::str::FromStr;

use cosmwasm_std::{
    Coin, DecCoin, Decimal, Decimal256, Empty, QuerierWrapper, SystemError, SystemResult, Uint128,
    Uint256, from_json,
    testing::{MockQuerier, mock_dependencies, mock_env},
    to_json_binary,
};
use cw20::TokenInfoResponse;

use crate::{
    execute,
    state::{
        BurnQueue, MintQueue, PARAMETERS, REWARD_BALANCE, STATE, SUPPLY_QUEUE, State, SupplyQueue,
        VALIDATORS_REGISTRY, Validator, ValidatorsRegistry, WITHDRAW_REWARD_QUEUE,
        WithdrawRewardQueue,
    },
    tests::mock_parameters,
    utils::calc::*,
};

#[test]
fn test_calculate_query_bounds() {
    assert_eq!(calculate_query_bounds(None, None), (1, 50));
    assert_eq!(calculate_query_bounds(Some(200), None), (200, 249));
    assert_eq!(calculate_query_bounds(None, Some(200)), (1, 50));
    assert_eq!(calculate_query_bounds(Some(100), Some(300)), (100, 149));
    assert_eq!(calculate_query_bounds(Some(2), Some(10)), (2, 10));
    assert_eq!(calculate_query_bounds(Some(1000), Some(2000)), (1000, 1049));
    assert_eq!(calculate_query_bounds(Some(200), Some(210)), (200, 210));
}

#[test]
fn test_calculate_staking_token_from_rate() {
    let stake_amount = Uint128::new(112_382);
    assert_eq!(
        calculate_staking_token_from_rate(stake_amount, Decimal::from_ratio(1_u128, 2_u128)),
        stake_amount * Uint128::new(2)
    );
    assert_eq!(
        calculate_staking_token_from_rate(stake_amount, Decimal::from_str("1.0").unwrap()),
        stake_amount
    );
}

#[test]
fn test_calculate_native_token_from_staking_token() {
    let staking_token = Uint128::new(112_382);
    assert_eq!(
        calculate_native_token_from_staking_token(
            staking_token,
            Decimal::from_ratio(1_u128, 2_u128)
        ),
        staking_token / Uint128::new(2)
    );

    let decimal_fractional: u128 = 1_000_000_000_000_000_000u128;
    let staking_token = Uint128::new(decimal_fractional);
    assert_eq!(
        calculate_native_token_from_staking_token(
            staking_token,
            Decimal::from_ratio(1u128, staking_token)
        ),
        Uint128::one()
    );
    let staking_token = Uint128::new(decimal_fractional + 1);
    assert_eq!(
        calculate_native_token_from_staking_token(
            staking_token,
            Decimal::from_ratio(1u128, staking_token)
        ),
        Uint128::zero() // Not enough precision
    );
}

#[test]
fn test_check_slippage() {
    // Same value
    assert!(check_slippage(Uint128::new(10), Uint128::new(10), Decimal::zero()).is_ok());

    // Good - lower bound
    assert!(check_slippage(Uint128::new(98), Uint128::new(100), Decimal::percent(2)).is_ok());
    // Fails - lower bound
    assert!(check_slippage(Uint128::new(98), Uint128::new(100), Decimal::percent(1)).is_err());

    // Good - upper bound
    assert!(check_slippage(Uint128::new(100), Uint128::new(105), Decimal::percent(5)).is_ok());
    // Fails - upper bound
    assert!(check_slippage(Uint128::new(100), Uint128::new(105), Decimal::percent(4)).is_err());
}

#[test]
fn test_to_uint128() {
    let amount = 123_050;
    assert_eq!(
        to_uint128(Uint256::from_u128(amount)),
        Ok(Uint128::new(amount))
    );
}

#[test]
fn test_total_lst_supply() {
    let mut querier = MockQuerier::default();
    let total_supply = Uint128::new(100_000);
    querier.update_wasm(move |wasm_query| {
        let unsupported_err = SystemResult::Err(SystemError::Unknown {});
        match wasm_query {
            cosmwasm_std::WasmQuery::Smart {
                contract_addr: _,
                msg,
            } => {
                let cw20_msg: cw20::Cw20QueryMsg = from_json(msg).unwrap();
                match cw20_msg {
                    cw20::Cw20QueryMsg::TokenInfo {} => {
                        let response = TokenInfoResponse {
                            name: String::default(),
                            symbol: String::default(),
                            decimals: u8::default(),
                            total_supply,
                        };
                        let bin = to_json_binary(&response).unwrap();
                        SystemResult::Ok(cosmwasm_std::ContractResult::Ok(bin))
                    }
                    _ => unsupported_err,
                }
            }
            _ => unsupported_err,
        }
    });
    let querier_wrapper = QuerierWrapper::<Empty>::new(&querier);
    assert_eq!(
        total_lst_supply(querier_wrapper, "cw20".to_string()).unwrap(),
        total_supply
    );
}

#[test]
fn test_normalize_supply_queue() {
    let mint_queue = vec![
        MintQueue {
            amount: Uint128::new(40),
            block: 700,
        },
        MintQueue {
            amount: Uint128::new(50),
            block: 650,
        },
        MintQueue {
            amount: Uint128::new(20),
            block: 730,
        },
    ];

    let burn_queue = vec![
        BurnQueue {
            amount: Uint128::new(10),
            block: 700,
        },
        BurnQueue {
            amount: Uint128::new(20),
            block: 730,
        },
        BurnQueue {
            amount: Uint128::new(30),
            block: 650,
        },
    ];

    let mut supply_queue = SupplyQueue {
        mint: mint_queue,
        burn: burn_queue,
        epoch_period: 3600,
    };

    let current_block = 740;
    normalize_supply_queue(&mut supply_queue, current_block);
    println!(">> new_supply_queue::: {supply_queue:?} ");
}

#[test]
fn test_normalize_total_supply() {
    let mint_queue = vec![
        MintQueue {
            amount: Uint128::new(40),
            block: 700,
        },
        MintQueue {
            amount: Uint128::new(50),
            block: 171_168_541,
        },
        MintQueue {
            amount: Uint128::new(20),
            block: 700,
        },
    ];

    let burn_queue = vec![
        BurnQueue {
            amount: Uint128::new(10),
            block: 700,
        },
        BurnQueue {
            amount: Uint128::new(20),
            block: 171_168_541,
        },
        BurnQueue {
            amount: Uint128::new(30),
            block: 700,
        },
    ];

    let mut supply_queue = SupplyQueue {
        mint: mint_queue,
        burn: burn_queue,
        epoch_period: 360,
    };

    let current_supply = Uint128::from(20000u128);
    let current_block = 1000;

    normalize_supply_queue(&mut supply_queue, current_block);

    let new_supply = normalize_total_supply(current_supply, &supply_queue.mint, &supply_queue.burn);
    println!("current_supply :{current_supply} >> new_supply::: {new_supply} ");
}

#[test]
fn test_calculate_dust_distribution() {
    assert!(calculate_dust_distribution(Uint128::zero(), Uint128::zero()).is_empty());
    assert!(calculate_dust_distribution(Uint128::new(1000), Uint128::zero()).is_empty());
    assert_eq!(
        calculate_dust_distribution(Uint128::zero(), Uint128::new(10)).len(),
        10
    );
    assert!(
        calculate_dust_distribution(Uint128::zero(), Uint128::new(10))
            .iter()
            .all(cosmwasm_std::Uint128::is_zero)
    );
    assert_eq!(
        calculate_dust_distribution(Uint128::new(10), Uint128::new(2)),
        Vec::from([Uint128::new(5), Uint128::new(5)])
    );
    assert_eq!(
        calculate_dust_distribution(Uint128::new(9), Uint128::new(2)),
        Vec::from([Uint128::new(5), Uint128::new(4)])
    );
    assert_eq!(
        calculate_dust_distribution(Uint128::new(11), Uint128::new(5)),
        Vec::from([
            Uint128::new(3),
            Uint128::new(2),
            Uint128::new(2),
            Uint128::new(2),
            Uint128::new(2),
        ])
    );
    let big_dust_amount = Uint128::new(12_340_123_203_498_754_234_792_834);
    assert_eq!(
        calculate_dust_distribution(big_dust_amount, Uint128::new(1500))
            .iter()
            .sum::<Uint128>(),
        big_dust_amount
    );
}

#[test]
fn test_staker_undelegation_with_dust_distribution() {
    let total_received_amount = Uint128::from_str("2022599").unwrap();

    let total_liquid_stake: Uint128 = Uint128::from_str("1949557").unwrap();

    let unbond_record_1 = crate::state::UnbondRecord {
        id: 101,
        height: 730_899,
        sender: "bbn1vnglhewf3w66cquy6hr7urjv3589srheqj3myz".into(),
        staker: "bbn1vnglhewf3w66cquy6hr7urjv3589srheqj3myz".into(),
        channel_id: None,
        amount: Uint128::new(540_000_u128),
        released_height: 0,
        released: false,
        batch_id: 27,
        recipient: None,
        recipient_channel_id: None,
    };

    let unbond_record_2 = crate::state::UnbondRecord {
        id: 101,
        height: 730_899,
        sender: "bbn1vnglhewf3w66cquy6hr7urjv3589srheqj3myz".into(),
        staker: "bbn1yj3h4tjw8s6n0cd6jmc0s9pqmud57yk5hf2nvf".into(),
        channel_id: None,
        amount: Uint128::new(409_557_u128),
        released_height: 0,
        released: false,
        batch_id: 27,
        recipient: None,
        recipient_channel_id: None,
    };

    let unbond_record_3 = crate::state::UnbondRecord {
        id: 101,
        height: 730_899,
        sender: "bbn1vnglhewf3w66cquy6hr7urjv3589srheqj3myz".into(),
        staker: "bbn132ltlddr9gkun8kgrefquem2w754kpy8z5j4wx".into(),
        channel_id: None,
        amount: Uint128::new(1_000_000_u128),
        released_height: 0,
        released: false,
        batch_id: 27,
        recipient: None,
        recipient_channel_id: None,
    };

    let mut unbond_records = vec![unbond_record_1, unbond_record_2, unbond_record_3];

    let mut store = cosmwasm_std::testing::MockStorage::new();

    let (_, unbond_record_ids, total_released_amount) =
        crate::utils::delegation::get_staker_undelegation(
            &mut store,
            total_received_amount,
            &mut unbond_records,
            total_liquid_stake,
            1000,
        )
        .unwrap();

    assert_eq!(total_received_amount, total_released_amount);
    assert_eq!(
        unbond_record_ids,
        unbond_records.iter().map(|u| u.id).collect::<Vec<u64>>()
    );
}

#[test]
fn test_normalize_reward_balance() {
    let mut deps = mock_dependencies();

    REWARD_BALANCE
        .save(&mut deps.storage, &Uint128::zero())
        .unwrap();
    crate::state::WITHDRAW_REWARD_QUEUE
        .save(&mut deps.storage, &vec![])
        .unwrap();

    SUPPLY_QUEUE
        .save(
            &mut deps.storage,
            &SupplyQueue {
                mint: vec![],
                burn: vec![],
                epoch_period: 360,
            },
        )
        .unwrap();

    // 1st bond at block 300
    // query reward on this block and assume we set to 100
    let block = 300;
    let unclaimed_reward_balance: u128 = 100;
    normalize_reward_balance(&mut deps.storage, block, unclaimed_reward_balance.into()).unwrap();
    // 2nd bond at block 350
    // query reward on this block and assume we set to 150
    let block = 350;
    let unclaimed_reward_balance: u128 = 150;
    normalize_reward_balance(&mut deps.storage, block, unclaimed_reward_balance.into()).unwrap();

    let reward_balance = REWARD_BALANCE.load(&deps.storage).unwrap();
    let reward_queue = WITHDRAW_REWARD_QUEUE.load(&deps.storage).unwrap();

    println!("==== after 2x transactions ==== at block 300 & 350");
    println!("reward_balance : {reward_balance}");
    println!("reward queue: {reward_queue:?}");
    assert_eq!(reward_balance, Uint128::new(0u128));

    // EPOCH HAPPEN at 401

    // 3rd bond at block 500
    // query reward on this block and assume we set to 100
    let block = 500;
    let unclaimed_reward_balance: u128 = 200;
    normalize_reward_balance(&mut deps.storage, block, unclaimed_reward_balance.into()).unwrap();
    // 4rd bond at block 350
    // query reward on this block and assume we set to 150
    let block = 550;
    let unclaimed_reward_balance: u128 = 250;
    normalize_reward_balance(&mut deps.storage, block, unclaimed_reward_balance.into()).unwrap();

    let reward_balance = REWARD_BALANCE.load(&deps.storage).unwrap();
    let reward_queue = WITHDRAW_REWARD_QUEUE.load(&deps.storage).unwrap();

    println!("==== after 2x transactions ==== at block 500 & 550");
    println!("reward_balance : {reward_balance}");
    println!("reward queue: {reward_queue:?}");
    assert_eq!(reward_balance, Uint128::new(150u128));

    // EPOCH HAPPEN at 801

    // 5rd bond at block 850
    // query reward on this block and assume we set to 100
    let block = 850;
    let unclaimed_reward_balance: u128 = 250;
    normalize_reward_balance(&mut deps.storage, block, unclaimed_reward_balance.into()).unwrap();
    // 6rd bond at block 870
    // query reward on this block and assume we set to 150
    let block = 870;
    let unclaimed_reward_balance: u128 = 260;
    normalize_reward_balance(&mut deps.storage, block, unclaimed_reward_balance.into()).unwrap();
    // 7rd bond at block 900
    // query reward on this block and assume we set to 150
    let block = 900;
    let unclaimed_reward_balance: u128 = 270;
    normalize_reward_balance(&mut deps.storage, block, unclaimed_reward_balance.into()).unwrap();

    let reward_balance = REWARD_BALANCE.load(&deps.storage).unwrap();
    let reward_queue = WITHDRAW_REWARD_QUEUE.load(&deps.storage).unwrap();

    assert_eq!(reward_balance, Uint128::new(400u128));
    println!("==== after 3x transactions ==== at block 850,870 & 900");
    println!("reward_balance : {reward_balance}");
    println!("reward queue: {reward_queue:?}");
}

#[test]
fn test_normalize_withdraw_reward_queue() {
    let current_block_height = 1_028_176;
    let current_reward_balance = Uint128::zero();

    let reward_amount = Uint128::new(172_786_u128);

    let reward_queue = vec![WithdrawRewardQueue {
        amount: reward_amount,
        block: 1_028_039,
    }];

    let epoch_period = 360;

    let (new_balance, new_queue) = normalize_withdraw_reward_queue(
        current_block_height,
        current_reward_balance,
        reward_queue,
        epoch_period,
    );

    println!("new_balance: {new_balance}");
    println!("new_queue: {new_queue:?}");
    assert_eq!(new_balance, reward_amount);
    assert_eq!(new_queue, vec![]);
}

#[test]
#[allow(clippy::too_many_lines)]
fn test_normalize_reward() {
    let mut deps = mock_dependencies();
    let delegator = deps.api.addr_make("delegator");
    let state = State {
        exchange_rate: Decimal::from_str("1.1").unwrap(),
        total_bond_amount: Uint128::new(20_000),
        total_delegated_amount: Uint128::new(15_000),
        total_supply: Uint128::new(10_000),
        bond_counter: 5,
        last_bond_time: 50000,
    };
    STATE.save(deps.as_mut().storage, &state).unwrap();
    let supply_queue = SupplyQueue {
        mint: vec![],
        burn: vec![],
        epoch_period: 200,
    };
    SUPPLY_QUEUE
        .save(deps.as_mut().storage, &supply_queue)
        .unwrap();
    let params = mock_parameters();
    PARAMETERS.save(deps.as_mut().storage, &params).unwrap();
    REWARD_BALANCE
        .save(deps.as_mut().storage, &Uint128::zero())
        .unwrap();

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
    let registry = ValidatorsRegistry {
        validators: validators.clone(),
    };
    VALIDATORS_REGISTRY
        .save(deps.as_mut().storage, &registry)
        .unwrap();
    let validators_cosm: &[cosmwasm_std::Validator; 2] = &[
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

    let queue = vec![WithdrawRewardQueue {
        amount: Uint128::new(100),
        block: 900,
    }];

    WITHDRAW_REWARD_QUEUE
        .save(deps.as_mut().storage, &queue)
        .unwrap();
    let mut env = mock_env();
    env.block.height = 1100;

    let res = execute::normalize_reward(deps.as_mut(), env);
    assert!(res.is_err());

    let queue = vec![WithdrawRewardQueue {
        amount: Uint128::new(100),
        block: 1100,
    }];

    WITHDRAW_REWARD_QUEUE
        .save(deps.as_mut().storage, &queue)
        .unwrap();
    let mut env = mock_env();
    env.block.height = 1150;

    let res = execute::normalize_reward(deps.as_mut(), env);
    println!("res: {res:?}");
    assert!(res.is_err());

    let mut env = mock_env();
    env.block.height = 1199;

    let res = execute::normalize_reward(deps.as_mut(), env);
    assert!(res.is_ok());

    let mut env = mock_env();
    env.block.height = 1200;

    let res = execute::normalize_reward(deps.as_mut(), env);
    println!("res: {res:?}");
    assert!(res.is_ok());

    let mut env: cosmwasm_std::Env = mock_env();
    env.block.height = 1400;

    let res = execute::normalize_reward(deps.as_mut(), env);
    println!("res: {res:?}");
    assert!(res.is_err());
}
