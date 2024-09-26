use crate::relay::send_to_evm;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, Addr, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StakingMsg, Uint128,
};
use token_factory_api::TokenFactoryMsg;

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::state::{
    Parameters, State, Validator, ValidatorsRegistry, PARAMETERS, STATE, VALIDATORS_REGISTRY,
};
use crate::utils::{
    calculate_token_from_rate, get_actual_total_bonded, get_actual_total_reward,
    get_mock_total_reward,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:evm_union_liquid_staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let mut validators: Vec<Validator> = vec![];
    for validator_addr in msg.validators {
        validators.push({
            Validator {
                address: validator_addr.to_string(),
            }
        })
    }

    let reg = ValidatorsRegistry { validators };
    VALIDATORS_REGISTRY.save(deps.storage, &reg)?;

    let params = Parameters {
        underlying_coin_denom: msg.underlying_coin_denom,
        liquidstaking_denom: msg.liquidstaking_denom,
        liquidstaking_denom_address: msg.liquidstaking_denom_address.to_string(),
        ucs01_channel: msg.ucs01_channel,
        ucs01_relay_contract: msg.ucs01_relay_contract,
    };
    PARAMETERS.save(deps.storage, &params)?;

    let state = State {
        exchange_rate: Decimal::one(),
        total_delegated_amount: Uint128::new(0),
        total_bond_amount: Uint128::new(0),
        total_lst_supply: Uint128::new(0),
        bond_counter: 0,
        last_bond_time: 0,
    };
    STATE.save(deps.storage, &state)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    match msg {
        ExecuteMsg::Bond {} => execute_bond(deps, env, info),
        ExecuteMsg::Transfer { amount, receiver } => {
            execute_transfer(deps, env, info, amount, receiver)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute_bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom;
    let sender = info.sender;

    // coin must have be sent along with transaction and it should be in underlying coin denom
    if info.funds.len() > 1usize {
        return Err(ContractError::InvalidAsset {});
    }

    // coin must have be sent along with transaction and it should be in underlying coin denom
    let payment = info
        .funds
        .iter()
        .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
        .ok_or_else(|| ContractError::NoAsset {})?;

    let total_validators = Uint128::from(validators_reg.validators.len() as u32);

    let delegate_amount = payment.amount / total_validators;
    let remaining_amount = payment.amount % total_validators;

    let mut msgs: Vec<CosmosMsg<TokenFactoryMsg>> = vec![];
    for (pos, validator) in validators_reg.validators.iter().enumerate() {
        let amount = Coin {
            amount: delegate_amount.clone(),
            denom: coin_denom.to_string(),
        };
        let mut staking_msg: CosmosMsg<TokenFactoryMsg> =
            CosmosMsg::Staking(StakingMsg::Delegate {
                validator: validator.address.to_string(),
                amount,
            });

        if pos == 0 {
            let amount = Coin {
                amount: delegate_amount + remaining_amount,
                denom: coin_denom.to_string(),
            };
            staking_msg = CosmosMsg::Staking(StakingMsg::Delegate {
                validator: validator.address.to_string(),
                amount,
            });
        }
        msgs.push(staking_msg.into());
    }

    let delegator = env.contract.address;
    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    // logic to mint token and update the supply and total_bond_amount
    let mut state = STATE.load(deps.storage)?;
    let total_bond_amount: Uint128;

    if !cfg!(test) {
        let delegated_amount = get_actual_total_bonded(
            deps.querier,
            delegator.to_string(),
            coin_denom.clone(),
            validators_list.clone(),
        );
        state.total_delegated_amount = delegated_amount;
        total_bond_amount = delegated_amount
            + get_actual_total_reward(
                deps.querier,
                delegator.to_string(),
                coin_denom.clone(),
                validators_list,
            );
    } else {
        total_bond_amount = get_mock_total_reward(state.total_bond_amount);
    }

    let mut current_exchange_rate = state.exchange_rate;

    if !total_bond_amount.is_zero() && !state.total_lst_supply.is_zero() {
        current_exchange_rate = Decimal::from_ratio(total_bond_amount, state.total_lst_supply);
    }

    let mint_amount = calculate_token_from_rate(payment.amount, current_exchange_rate);

    let total_lst_supply = state.total_lst_supply;

    // after update exchange rate we update the state
    state.bond_counter = state.bond_counter + 1;
    state.total_bond_amount = total_bond_amount + payment.amount;
    state.total_lst_supply = total_lst_supply + mint_amount;
    state.total_delegated_amount += payment.amount;
    state.update_exchange_rate();

    STATE.save(deps.storage, &state)?;

    // Start to mint according to staked token
    let msg = TokenFactoryMsg::MintTokens {
        denom: params.liquidstaking_denom.clone(),
        amount: mint_amount,
        mint_to_address: params.liquidstaking_denom_address,
    };

    if !cfg!(test) {
        msgs.push(msg.into());
    }

    let res: Response<TokenFactoryMsg> = Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "mint"),
        attr("from", sender),
        attr("minted", mint_amount),
        attr("exchange_rate", state.exchange_rate.to_string()),
    ]);

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute_transfer(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    amount: Coin,
    receiver: Addr,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;

    let funds = vec![amount];
    let msg: CosmosMsg<TokenFactoryMsg> = send_to_evm(
        params.ucs01_relay_contract,
        params.ucs01_channel,
        receiver.to_string(),
        funds,
    )?
    .into();

    let res: Response<TokenFactoryMsg> = Response::new().add_message(msg);
    Ok(res)
}

// let funds = vec![{
//     Coin {
//         amount: mint_amount,
//         denom: params.liquidstaking_denom,
//     }
// }];

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let ver = cw2::get_contract_version(deps.storage)?;
    // ensure we are migrating from a compatible contract
    if ver.contract != CONTRACT_NAME {
        return Err(ContractError::InvalidContractName {});
    }
    // note: it's better to do a proper semver comparison, but a string comparison *usually* works    #[allow(clippy::cmp_owned)]
    if ver.version >= CONTRACT_VERSION.to_string() {
        return Err(ContractError::InvalidContractVersion {
            message: "Current version is equal or newer than the new contract code".to_string(),
        });
    } // set the new version
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
