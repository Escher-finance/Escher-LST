#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{attr, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StakingMsg, Uint128};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{
    Parameters, State, Validator, ValidatorsRegistry, PARAMETERS, STATE, VALIDATORS_REGISTRY,
};

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
) -> Result<Response, ContractError> {
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
    };
    PARAMETERS.save(deps.storage, &params)?;

    let state = State {
        exchange_rate: Decimal::one(),
        total_bond_amount: Uint128::new(0),
        last_unbonded_time: 0,
    };
    STATE.save(deps.storage, &state)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
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

    let mut msgs: Vec<CosmosMsg> = vec![];
    for (pos, validator) in validators_reg.validators.iter().enumerate() {
        let amount = Coin{
            amount: delegate_amount.clone(),
            denom: coin_denom.to_string(),
        };
        let mut staking_msg = StakingMsg::Delegate {
            validator: validator.address.to_string(),
            amount,
        };
        if pos == 0 {
            let amount = Coin{
                amount: delegate_amount + remaining_amount,
                denom: coin_denom.to_string(),
            };
            staking_msg = StakingMsg::Delegate {
                validator: validator.address.to_string(),
                amount,
            };
        }
        msgs.push(CosmosMsg::Staking(staking_msg));
    }

    let res = Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "mint"),
        attr("from", sender),
        attr("bonded", payment.clone().amount),
    ]);

    Ok(res)
}
