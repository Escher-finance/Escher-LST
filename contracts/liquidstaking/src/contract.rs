#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StakingMsg, Uint128,
};
use token_factory_api::TokenFactoryMsg;
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{
    Parameters, State, Validator, ValidatorsRegistry, PARAMETERS, STATE, VALIDATORS_REGISTRY,
};
use crate::utils::{decimal_division, get_actual_total_bonded, get_actual_total_reward, get_mock_total_reward};

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:evm-union-liquid-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
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
        staked_token_denom: msg.staked_token_denom,
        staked_token_denom_address: msg.staked_token_denom_address.to_string(),
    };
    PARAMETERS.save(deps.storage, &params)?;

    let state = State {
        exchange_rate: Decimal::one(),
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
    _msg: ExecuteMsg,
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

    // logic to mint token and update the supply and total_bond_amount
    let mut state = STATE.load(deps.storage)?;

    let mint_amount = decimal_division(payment.amount, state.exchange_rate);

    let delegator = env.contract.address;
    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();


    if !cfg!(test) {
        state.total_bond_amount = get_actual_total_bonded(
            deps.querier,
            delegator.to_string(),
            coin_denom.clone(),
            validators_list.clone(),
        ) + get_actual_total_reward(
            deps.querier,
            delegator.to_string(),
            coin_denom.clone(),
            validators_list,
        );
    } else {
        state.total_bond_amount = get_mock_total_reward(state.total_bond_amount);
    }

    let total_lst_supply = state.total_lst_supply;

    // after update exchange rate we update the state
    state.bond_counter = state.bond_counter + 1;
    state.total_bond_amount += payment.amount;
    state.update_exchange_rate(total_lst_supply, mint_amount);

    state.total_lst_supply = total_lst_supply + mint_amount;
    STATE.save(deps.storage, &state)?;

    // Start to mint according to staked token
    let msg = TokenFactoryMsg::MintTokens {
        denom: params.staked_token_denom,
        amount: mint_amount,
        mint_to_address: params.staked_token_denom_address,
    };

    if !cfg!(test) {
        msgs.push(msg.into());
    }

    let res: Response<TokenFactoryMsg> = Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "mint"),
        attr("from", sender),
        attr("bonded", mint_amount),
    ]);

    Ok(res)
}
