use cosmos_sdk_proto::{
    Any,
    cosmos::{authz::v1beta1::MsgExec, base::v1beta1::Coin as ProtoCoin},
    cosmwasm::wasm::v1::MsgExecuteContract,
    traits::Message,
};
use cosmwasm_std::{AnyMsg, Binary, Coin, CosmosMsg, Timestamp, Uint128, to_json_binary};
use unionlabs_primitives::{Bytes, H256};

use crate::{error::ContractError, zkgm::protocol::ucs03_transfer};

pub fn cosmos_msg_for_contract_execution(
    granter: String,
    grantee: String,
    target_contract_address: String,
    msg: Binary,
    funds: Vec<Coin>,
) -> Result<CosmosMsg, ContractError> {
    let proto_funds: Vec<ProtoCoin> = funds
        .iter()
        .map(|c| ProtoCoin {
            denom: c.denom.clone(),
            amount: c.amount.to_string(),
        })
        .collect();
    let execute_contract = MsgExecuteContract {
        sender: granter,
        contract: target_contract_address,
        msg: msg.to_vec(),
        funds: proto_funds,
    };

    let execute_any = Any::from_msg(&execute_contract);

    if execute_any.is_err() {
        return Err(ContractError::EncodeAnyMsgError {});
    };

    let execute_msg = MsgExec {
        grantee,
        msgs: vec![execute_any.unwrap()],
    };

    let execute_stargate = CosmosMsg::Any(AnyMsg {
        type_url: "/cosmos.authz.v1beta1.MsgExec".to_string(),
        value: Binary::from(execute_msg.encode_to_vec()),
    });

    Ok(execute_stargate)
}

pub fn get_authz_increase_allowance_msg(
    granter: String,
    grantee: String,
    cw20_contract: String,
    spender: String,
    amount: Uint128,
    funds: Vec<Coin>,
) -> Result<CosmosMsg, ContractError> {
    let allowance_msg = cw20::Cw20ExecuteMsg::IncreaseAllowance {
        spender,
        amount,
        expires: None,
    };

    let allow_bin = to_json_binary(&allowance_msg).unwrap();

    cosmos_msg_for_contract_execution(granter, grantee, cw20_contract, allow_bin, funds)
}

#[allow(clippy::too_many_arguments)]
pub fn get_authz_ucs03_transfer(
    cw20_contract: String,
    granter: String,
    grantee: String,
    time: Timestamp,
    ucs03_contract_addr: String,
    channel_id: u32,
    receiver: Bytes,
    base_token: String,
    base_amount: Uint128,
    quote_token: Bytes,
    quote_amount: Uint128,
    funds: Vec<Coin>,
    salt: H256,
) -> Result<CosmosMsg, ContractError> {
    let ucs03_transfer_msg_bin = ucs03_transfer(
        cw20_contract,
        time,
        channel_id,
        granter.clone(), // granter is transfer handler
        receiver,
        base_token,
        base_amount,
        quote_token,
        quote_amount,
        salt,
    )?;

    cosmos_msg_for_contract_execution(
        granter,
        grantee,
        ucs03_contract_addr,
        ucs03_transfer_msg_bin,
        funds,
    )
}
