use cosmwasm_std::{
    instantiate2_address, to_json_binary, Addr, Binary, CosmosMsg, Decimal, DepsMut, Env, WasmMsg,
};

use crate::{error::ContractError, msg::InstantiateRewardMsg};

#[allow(clippy::too_many_arguments)]
pub fn instantiate2(
    deps: &DepsMut,
    env: &Env,
    code_id: u64,
    salt: impl Into<String>,
    label: impl Into<String>,
    instantiate_msg: Binary,
    admin: Option<String>,
) -> Result<(CosmosMsg, Addr), ContractError> {
    let salt = salt.into();
    let code_info = deps.querier.query_wasm_code_info(code_id)?;
    let creator_cannonical = deps.api.addr_canonicalize(env.contract.address.as_str())?;
    let contract_addr = deps.api.addr_humanize(&instantiate2_address(
        code_info.checksum.as_slice(),
        &creator_cannonical,
        salt.as_bytes(),
    )?)?;

    let code_info_result = deps.querier.query_wasm_code_info(code_id);

    if let Err(code_info_err) = code_info_result {
        return Err(ContractError::InvalidCodeID {
            message: format!("Wallet code id : {code_id} not found, {code_info_err:?}"),
        });
    }

    let instantiate_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Instantiate2 {
        code_id,
        msg: instantiate_msg,
        funds: vec![],
        label: label.into(),
        admin,
        salt: salt.as_bytes().into(),
    });

    Ok((instantiate_msg, contract_addr))
}

#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
pub fn create_reward(
    deps: &DepsMut,
    env: &Env,
    salt: String,
    reward_code_id: u64,
    lst_contract: Addr,
    revenue_receiver: Addr,
    fee_rate: Decimal,
    coin_denom: String,
) -> Result<(CosmosMsg, Addr), ContractError> {
    let reward_label: String = format!("reward-instance-{salt}");
    let instantiate_msg = InstantiateRewardMsg {
        lst_contract: lst_contract.clone(),
        fee_receiver: revenue_receiver,
        fee_rate,
        coin_denom,
    };

    let (vote_create_msg, vote_address) = instantiate2(
        deps,
        env,
        reward_code_id,
        salt,
        reward_label,
        to_json_binary(&instantiate_msg)?,
        Some(lst_contract.to_string()),
    )?;

    Ok((vote_create_msg, vote_address))
}
