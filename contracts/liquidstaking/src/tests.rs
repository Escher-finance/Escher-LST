use std::str::FromStr;

use crate::contract::execute;
use crate::contract::instantiate;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::query::query;
use crate::state::{
    increment_tokens, unbond_record, State, UnbondRecord, Validator as EscherValidator,
    ValidatorsRegistry,
};
use crate::token_factory_api::TokenFactoryMsg;
use crate::utils::{
    calculate_native_token_from_staking_token, calculate_staking_token_from_rate,
    get_mock_total_reward,
};
use crate::ContractError;
use cosmwasm_std::testing::{message_info, mock_dependencies_with_balance, mock_env, MockApi};
use cosmwasm_std::{
    coins, from_json, Addr, Coin, Decimal, DepsMut, Env, MemoryStorage, Order, Response, StdError,
    Timestamp, Uint128, Validator,
};
use cw_multi_test::{
    App, AppBuilder, BankKeeper, Contract, ContractWrapper, Executor, FailingModule, StakingInfo,
    WasmKeeper,
};

pub fn make_addr(addr: &str) -> String {
    let api = MockApi::default();
    api.addr_make(addr).to_string()
}

fn set_up(
    deps: DepsMut,
    env: Env,
    validators: Vec<EscherValidator>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let denom_name: String = "muno".to_string();
    let receiver = Addr::unchecked("receiver");

    let cw20_contract = Addr::unchecked("cw20_contract");
    let ucs01_relay_contract = Addr::unchecked("relay_contract");
    let msg = InstantiateMsg {
        underlying_coin_denom: denom_name.clone(),
        validators,
        liquidstaking_denom: denom_name.clone(),
        ucs01_channel: "channel-01".to_string(),
        ucs01_relay_contract: ucs01_relay_contract.to_string(),
        fee_rate: Decimal::from_str("0.1").unwrap(),
        revenue_receiver: receiver.to_string(),
        unbonding_time: 1000u64,
        cw20_address: Some(cw20_contract),
    };

    let creator = MockApi::default().addr_make("owner");
    let info = message_info(&creator, &coins(2, denom_name.as_str()));
    let res = instantiate(deps, env, info, msg).unwrap();
    Ok(res)
}

pub type StakingApp = App<
    BankKeeper,
    MockApi,
    MemoryStorage,
    FailingModule<TokenFactoryMsg, cosmwasm_std::Empty, cosmwasm_std::Empty>,
    WasmKeeper<TokenFactoryMsg, cosmwasm_std::Empty>,
>;

const VALIDATOR_ONE_ADDRESS: &str = "validator_one";
const STAKING_DENOM: &str = "TOKEN";
const LIQUID_STAKING_DENOM: &str = "stTOKEN";
const SUPPLY: u128 = 500_000_000u128;

pub fn liquid_staking_contract() -> Box<dyn Contract<TokenFactoryMsg>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}

fn setup_contract() -> (Addr, StakingApp, Addr) {
    let owner: Addr = Addr::unchecked(make_addr("owner"));
    let validator_addr: Addr = Addr::unchecked(make_addr(VALIDATOR_ONE_ADDRESS));

    let mut app: StakingApp = AppBuilder::new_custom().build(|router, api, storage| {
        let env = mock_env();
        // Set the initial balances for USER
        router
            .bank
            .init_balance(
                storage,
                &owner,
                vec![Coin {
                    denom: STAKING_DENOM.to_string(),
                    amount: Uint128::from(SUPPLY),
                }],
            )
            .unwrap();

        // Setup staking module for the correct mock data.
        router
            .staking
            .setup(
                storage,
                StakingInfo {
                    bonded_denom: STAKING_DENOM.to_string(),
                    unbonding_time: 1, // in seconds
                    apr: Decimal::percent(10),
                },
            )
            .unwrap();

        let validator = Validator::create(
            validator_addr.to_string(),
            Decimal::zero(),
            Decimal::one(),
            Decimal::one(),
        );
        // Add mock validator
        router
            .staking
            .add_validator(api, storage, &env.block, validator)
            .unwrap();
    });

    let ls_code_id = app.store_code(liquid_staking_contract());

    let denom_name: String = STAKING_DENOM.to_string();
    let ls_denom_name: String = LIQUID_STAKING_DENOM.to_string();

    let val1 = EscherValidator {
        address: validator_addr.to_string(),
        weight: 1,
    };
    let validators: Vec<EscherValidator> = vec![val1];

    let receiver = Addr::unchecked(make_addr("receiver"));

    let cw20_contract = Addr::unchecked(make_addr("cw20_contract"));
    let ucs01_relay_contract = Addr::unchecked(make_addr("relay_contract"));
    let msg = InstantiateMsg {
        underlying_coin_denom: denom_name.clone(),
        validators,
        liquidstaking_denom: ls_denom_name,
        ucs01_channel: "channel-01".to_string(),
        ucs01_relay_contract: ucs01_relay_contract.to_string(),
        fee_rate: Decimal::from_str("0.1").unwrap(),
        revenue_receiver: receiver.to_string(),
        unbonding_time: 1000u64,
        cw20_address: Some(cw20_contract),
    };
    // Instantiate the multisig contract using its newly stored code id
    let ls_address = app
        .instantiate_contract(ls_code_id, owner.clone(), &msg, &[], "ls-test", None)
        .unwrap();

    (owner, app, ls_address)
}

#[test]
fn proper_instantiation() {
    let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
    let validator_addr: Addr = Addr::unchecked(VALIDATOR_ONE_ADDRESS);
    let val1 = EscherValidator {
        address: validator_addr.to_string(),
        weight: 1,
    };

    let env = mock_env();
    let res = set_up(deps.as_mut(), env, vec![val1]);
    let response = res.unwrap();
    assert_eq!(response.clone().messages.len(), 0);
}

#[test]
fn initial_query() {
    let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
    let validator_addr: Addr = Addr::unchecked(VALIDATOR_ONE_ADDRESS);
    let val1 = EscherValidator {
        address: validator_addr.to_string(),
        weight: 1,
    };
    let env = mock_env();
    let _ = set_up(deps.as_mut(), env.clone(), vec![val1.clone()]);

    let msg = QueryMsg::Validators {};
    let reg: ValidatorsRegistry = from_json(query(deps.as_ref(), env, msg).unwrap()).unwrap();
    assert_eq!(
        reg.validators.first().unwrap().address,
        validator_addr.to_string()
    );
}

#[test]
fn execute_bond() {
    let (owner, mut app, ls_contract_addr) = setup_contract();

    let bond_msg = ExecuteMsg::Bond {
        staker: Some(owner.to_string()),
        amount: None,
    };
    let res1 = app.execute_contract(owner.clone(), ls_contract_addr.clone(), &bond_msg, &[]);
    assert!(res1.is_err());

    let fund = Coin {
        amount: Uint128::new(10),
        denom: STAKING_DENOM.to_string(),
    };

    let res2 = app
        .execute_contract(
            owner.clone(),
            ls_contract_addr.clone(),
            &bond_msg,
            &vec![fund],
        )
        .unwrap();
    // println!("{:?}", res2);

    let msg = QueryMsg::State {};
    let res: Result<State, StdError> = app.wrap().query_wasm_smart(ls_contract_addr.clone(), &msg);
    let state: State = res.unwrap();
    // println!("{:?}", state);

    let fund2 = Coin {
        amount: Uint128::new(1000),
        denom: STAKING_DENOM.to_string(),
    };

    let res3 = app
        .execute_contract(
            owner.clone(),
            ls_contract_addr.clone(),
            &bond_msg,
            &vec![fund2],
        )
        .unwrap();
    //println!("{:?}", res3);

    let res2: Result<State, StdError> = app.wrap().query_wasm_smart(ls_contract_addr, &msg);
    let state2: State = res2.unwrap();
    //println!("{:?}", state2);
}

#[test]
fn exchange_rate_calculation() {
    let total_bond = Uint128::new(100);

    let a = Uint128::new(10);
    let b = Uint128::new(50);
    let exchange_rate = Decimal::from_ratio(a, b);
    //println!("{:?} / {:?}", total_bond, exchange_rate);

    let token = calculate_staking_token_from_rate(total_bond, exchange_rate);
    assert_eq!(token, Uint128::new(500));

    // - Rewards for 4 days: 1000 Union * 0.0274% * 4 = 1.096 Union
    // - Total staked Union + rewards (U + R): 1001.096 Union
    // - Total LUnion (L): 1000 LUnion

    // - New exchange rate: 1001.096 / 1000 = 1.001096 Union per LUnion
    // - Bob receives: 500 / 1.001096 = 499.45 LUnion

    let a = Uint128::new(1001096);
    let b = Uint128::new(1000000);
    let new_exchange_rate = Decimal::from_ratio(a, b);

    let bond_amount = Uint128::new(500000000);
    let mint_amount = calculate_staking_token_from_rate(bond_amount, new_exchange_rate);
    assert_eq!(mint_amount, Uint128::new(499452599));
}

#[test]
fn mock_total_reward() {
    let total_bond = Uint128::new(1000);
    let bond_with_reward = get_mock_total_reward(total_bond);
    assert_eq!(bond_with_reward, Uint128::new(1005));
}

#[test]
fn exchange_unbond_rate_calculation() {
    let staking_token = Uint128::new(100);

    let a = Uint128::new(110);
    let b = Uint128::new(100);
    let exchange_rate = Decimal::from_ratio(a, b);

    let token = calculate_native_token_from_staking_token(staking_token, exchange_rate);
    assert_eq!(token, Uint128::new(110));
}

#[test]
fn test_unbond_record() {
    let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

    let validator = deps.api.addr_make("validator");
    let env = mock_env();

    let val = EscherValidator {
        address: validator.to_string(),
        weight: 1,
    };
    let block_height = env.block.height;
    let _res = set_up(deps.as_mut(), env, vec![val]);

    let mut id = increment_tokens(&mut deps.storage).unwrap();
    //println!("{}", id);

    let staker = deps.api.addr_make("staker");
    let sender1 = deps.api.addr_make("sender1");
    let sender2 = deps.api.addr_make("sender2");

    let amount = Coin {
        amount: Uint128::new(10),
        denom: "TOKEN".to_string(),
    };

    let undelegate_amount = Coin {
        amount: Uint128::new(10),
        denom: "stTOKEN".to_string(),
    };

    let exchange_rate = Decimal::from_ratio(Uint128::new(110), Uint128::new(100));
    let ts = Timestamp::from_nanos(1_000_000_000);

    let mut history = UnbondRecord {
        id,
        height: block_height,
        staker: staker.to_string(),
        sender: sender1.to_string(),
        amount: amount.clone(),
        undelegate_amount: undelegate_amount.clone(),
        undelegations: vec![],
        exchange_rate: exchange_rate,
        created: ts,
        completion: ts,
        released: false,
        released_time: ts,
    };

    let _res = unbond_record().save(&mut deps.storage, id, &history);
    id = increment_tokens(&mut deps.storage).unwrap();
    history = UnbondRecord {
        id,
        height: block_height,
        staker: staker.to_string(),
        sender: sender2.to_string(),
        amount: amount.clone(),
        undelegate_amount,
        undelegations: vec![],
        exchange_rate: exchange_rate,
        created: ts,
        completion: ts,
        released: false,
        released_time: ts,
    };
    let _res2 = unbond_record().save(&mut deps.storage, id, &history);

    let unbonded_list1 = unbond_record()
        .idx
        .released
        .prefix("true".to_string())
        .range(&deps.storage, None, None, Order::Ascending)
        .map(|n| n.unwrap().1)
        .collect::<Vec<_>>();
    //println!("{:?}", unbonded_list1);

    let unbonded_list2 = unbond_record()
        .idx
        .released
        .prefix("false".to_string())
        .range(&deps.storage, None, None, Order::Ascending)
        .map(|n| n.unwrap().1)
        .collect::<Vec<_>>();

    //println!("{:?}", unbonded_list2);
}

#[test]
fn undelegate_amount_calculation() {
    let staking_token_amount = Uint128::new(1000);
    let total_bonded_amount = Uint128::new(1100);

    let current_exchange_rate = Decimal::from_ratio(total_bonded_amount, staking_token_amount);
    let unbond_amount = Uint128::new(500);
    let token = calculate_native_token_from_staking_token(unbond_amount, current_exchange_rate);
    println!("calculate_undelegate_amount: {:?}", token);
}

#[test]
fn split_revenue() {
    use crate::utils;
    let reward_amount = Uint128::new(251);
    let fee_rate = Decimal::from_str("0.1").unwrap();


    //check Decimal(100000000000000000)
    println!("fee_rate: {:?}", fee_rate);
    println!("{}", "Decimal(100000000000000000)");
    let (restake, fee) = utils::split_revenue(reward_amount, fee_rate);
    println!("split_revenue: {}, restake: {}, fee: {}", reward_amount, restake, fee);
}
