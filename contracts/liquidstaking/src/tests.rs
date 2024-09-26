use crate::contract::execute;
use crate::contract::instantiate;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::query::query;
use crate::state::{State, ValidatorsRegistry};
use crate::utils::{calculate_token_from_rate, get_mock_total_reward};
use crate::ContractError;
use cosmwasm_std::testing::{message_info, mock_dependencies_with_balance, mock_env, MockApi};
use cosmwasm_std::{
    coins, from_json, Addr, Coin, Decimal, DepsMut, Env, MemoryStorage, Response, StdError,
    Uint128, Validator,
};
use cw_multi_test::{
    App, AppBuilder, BankKeeper, Contract, ContractWrapper, Executor, FailingModule, StakingInfo,
    WasmKeeper,
};
use token_factory_api::TokenFactoryMsg;

fn set_up(
    deps: DepsMut,
    env: Env,
    validators: Vec<Addr>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let denom_name: String = "muno".to_string();
    let liquidstaking_denom_address = Addr::unchecked("lst_denom");

    let ucs01_relay_contract = Addr::unchecked("relay_contract");

    let msg = InstantiateMsg {
        underlying_coin_denom: denom_name.clone(),
        validators,
        liquidstaking_denom: denom_name.clone(),
        liquidstaking_denom_address,
        ucs01_channel: "channel-01".to_string(),
        ucs01_relay_contract: ucs01_relay_contract.to_string(),
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
const SUPPLY: u128 = 500_000_000u128;

pub fn liquid_staking_contract() -> Box<dyn Contract<TokenFactoryMsg>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}

fn setup_contract() -> (Addr, StakingApp, Addr) {
    let owner: Addr = Addr::unchecked("owner");
    let validator_addr: Addr = Addr::unchecked(VALIDATOR_ONE_ADDRESS);

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
    let validators: Vec<Addr> = vec![validator_addr];

    let ucs01_relay_contract = Addr::unchecked("relay_contract");
    let msg = InstantiateMsg {
        underlying_coin_denom: denom_name.clone(),
        validators,
        liquidstaking_denom: denom_name,
        liquidstaking_denom_address: owner.clone(),
        ucs01_channel: "channel-01".to_string(),
        ucs01_relay_contract: ucs01_relay_contract.to_string(),
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

    let validator = deps.api.addr_make("validator");
    let env = mock_env();
    let res = set_up(deps.as_mut(), env, vec![validator]);
    assert_eq!(res.unwrap().messages.len(), 0);
}

#[test]
fn initial_query() {
    let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
    let validator = deps.api.addr_make("validator");
    let env = mock_env();
    let _ = set_up(deps.as_mut(), env.clone(), vec![validator.clone()]);

    let msg = QueryMsg::Validators {};
    let config: ValidatorsRegistry = from_json(query(deps.as_ref(), env, msg).unwrap()).unwrap();
    assert_eq!(
        config.validators.first().unwrap().address,
        validator.to_string()
    );
}

#[test]
fn execute_bond() {
    let (owner, mut app, ls_contract_addr) = setup_contract();

    let bond_msg = ExecuteMsg::Bond {};
    let res1 = app.execute_contract(owner.clone(), ls_contract_addr.clone(), &bond_msg, &[]);
    //println!("{:?}", res1);
    assert_eq!(res1.is_err(), true);

    let fund = Coin {
        amount: Uint128::new(1000),
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
    println!("{:?}", res2);

    let msg = QueryMsg::State {};
    let res: Result<State, StdError> = app.wrap().query_wasm_smart(ls_contract_addr.clone(), &msg);
    let state: State = res.unwrap();
    println!("{:?}", state);

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
    println!("{:?}", res3);

    let res2: Result<State, StdError> = app.wrap().query_wasm_smart(ls_contract_addr, &msg);
    let state2: State = res2.unwrap();
    println!("{:?}", state2);
}

#[test]
fn exchange_rate_calculation() {
    let total_bond = Uint128::new(100);

    let a = Uint128::new(10);
    let b = Uint128::new(50);
    let exchange_rate = Decimal::from_ratio(a, b);
    //println!("{:?} / {:?}", total_bond, exchange_rate);

    let token = calculate_token_from_rate(total_bond, exchange_rate);
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
    let mint_amount = calculate_token_from_rate(bond_amount, new_exchange_rate);
    assert_eq!(mint_amount, Uint128::new(499452599));
}

#[test]
fn mock_total_reward() {
    let total_bond = Uint128::new(1000);
    let bond_with_reward = get_mock_total_reward(total_bond);
    assert_eq!(bond_with_reward, Uint128::new(1005));
}
