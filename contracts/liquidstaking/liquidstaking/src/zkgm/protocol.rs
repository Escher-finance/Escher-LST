use super::com::{
    Batch, Call, Instruction, SolverMetadata, TokenOrderV2, INSTR_VERSION_0, INSTR_VERSION_2,
    OP_BATCH, OP_CALL, OP_TOKEN_ORDER, TOKEN_ORDER_KIND_SOLVE,
};
use crate::msg::Ucs03ExecuteMsg;
use crate::types::ChannelId;
use crate::zkgm::com::ZkgmHubMsg;
use crate::ContractError;
use alloy::sol_types::SolValue;
use alloy_primitives::Bytes;
use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, StdError, Timestamp, Uint128, Uint64, WasmMsg,
};
use std::str::FromStr;
use unionlabs_primitives::H256;

pub fn ucs03_transfer(
    time: Timestamp,
    channel_id: u32,
    sender: String,
    receiver: String,
    base_token: String,
    base_amount: Uint128,
    quote_token: String,
    quote_amount: Uint128,
    salt: String,
) -> Result<Binary, ContractError> {
    let recipient_address = match Bytes::from_str(receiver.as_str()) {
        Ok(rec) => rec,
        Err(_) => {
            return Err(ContractError::InvalidAddress {
                kind: "recipient".into(),
                address: receiver,
                reason: "address must be in hex and starts with 0x".to_string(),
            })
        }
    };
    let quote_token = match Bytes::from_str(quote_token.as_str()) {
        Ok(token) => token,
        Err(_) => {
            return Err(ContractError::InvalidAddress {
                kind: "quote_token".into(),
                address: quote_token,
                reason: "address must be in hex and starts with 0x".to_string(),
            })
        }
    };

    let metadata = SolverMetadata {
        solverAddress: Vec::from(quote_token.clone()).into(),
        metadata: Default::default(),
    };
    let token_order_instruction = Instruction {
        version: INSTR_VERSION_2,
        opcode: OP_TOKEN_ORDER,
        operand: TokenOrderV2 {
            sender: sender.as_bytes().to_vec().into(),
            receiver: Vec::from(recipient_address).into(),
            base_token: base_token.as_bytes().to_vec().into(),
            base_amount: base_amount.u128().try_into().expect("u256>u128"),
            quote_token: Vec::from(quote_token).into(),
            quote_amount: quote_amount.u128().try_into().unwrap(),
            kind: TOKEN_ORDER_KIND_SOLVE,
            metadata: metadata.abi_encode_params().into(),
        }
        .abi_encode_params()
        .into(),
    };

    let timeout_timestamp_offset: u64 = 86400; // 1 day period
    let timeout_timestamp =
        Timestamp::from_nanos(time.plus_seconds(timeout_timestamp_offset).nanos());

    let salt: unionlabs_primitives::H256 = match unionlabs_primitives::H256::from_str(salt.as_str())
    {
        Ok(s) => s,
        Err(e) => {
            return Err(ContractError::Std(StdError::generic_err(format!(
                "failed to parse salt: {}, reason: {}",
                salt,
                e.to_string()
            ))))
        }
    };

    let relay_transfer_msg: Ucs03ExecuteMsg = Ucs03ExecuteMsg::Send {
        channel_id: ChannelId::from_raw(channel_id).unwrap(),
        timeout_height: Uint64::from(0u64),
        timeout_timestamp,
        salt,
        instruction: token_order_instruction.abi_encode_params().into(),
    };

    let transfer_relay_msg = to_json_binary(&relay_transfer_msg)?;
    Ok(transfer_relay_msg)
}

pub fn ucs03_transfer_and_call(
    time: Timestamp,
    channel_id: u32,
    sender: String,
    base_token: String,
    base_amount: Uint128,
    quote_token: String,
    quote_amount: Uint128,
    salt: String,
    hub_contract: String,
    contract_calldata: ZkgmHubMsg,
) -> Result<Binary, ContractError> {
    let recipient_address = match Bytes::from_str(hub_contract.as_str()) {
        Ok(rec) => rec,
        Err(_) => {
            return Err(ContractError::InvalidAddress {
                kind: "recipient".into(),
                address: hub_contract,
                reason: "address must be in hex and starts with 0x".to_string(),
            })
        }
    };
    let quote_token = match Bytes::from_str(quote_token.as_str()) {
        Ok(token) => token,
        Err(_) => {
            return Err(ContractError::InvalidAddress {
                kind: "quote_token".into(),
                address: quote_token,
                reason: "address must be in hex and starts with 0x".to_string(),
            })
        }
    };

    let metadata = SolverMetadata {
        solverAddress: Vec::from(quote_token.clone()).into(),
        metadata: Default::default(),
    };

    let fungible_order_instruction = Instruction {
        version: INSTR_VERSION_2,
        opcode: OP_TOKEN_ORDER,
        operand: TokenOrderV2 {
            sender: sender.as_bytes().to_vec().into(),
            receiver: Vec::from(recipient_address.clone()).into(),
            base_token: base_token.as_bytes().to_vec().into(),
            base_amount: base_amount.u128().try_into().expect("u256>u128"),
            quote_token: Vec::from(quote_token).into(),
            quote_amount: quote_amount.u128().try_into().unwrap(),
            kind: TOKEN_ORDER_KIND_SOLVE,
            metadata: metadata.abi_encode_params().into(),
        }
        .abi_encode_params()
        .into(),
    };

    let call_instruction = Instruction {
        version: INSTR_VERSION_0,
        opcode: OP_CALL,
        operand: Call {
            sender: sender.as_bytes().to_vec().into(),
            eureka: false,
            contract_address: Vec::from(recipient_address).into(),
            contract_calldata: contract_calldata.abi_encode_params().into(),
        }
        .abi_encode_params()
        .into(),
    };

    let batch_instruction = Instruction {
        version: INSTR_VERSION_0,
        opcode: OP_BATCH,
        operand: Batch {
            instructions: vec![fungible_order_instruction, call_instruction],
        }
        .abi_encode_params()
        .into(),
    };

    let timeout_timestamp_offset: u64 = 86400; // 1 day period
    let timeout_timestamp =
        Timestamp::from_nanos(time.plus_seconds(timeout_timestamp_offset).nanos());

    let salt: unionlabs_primitives::H256 = match unionlabs_primitives::H256::from_str(salt.as_str())
    {
        Ok(s) => s,
        Err(e) => {
            return Err(ContractError::Std(StdError::generic_err(format!(
                "failed to parse salt: {}, reason: {}",
                salt,
                e.to_string()
            ))))
        }
    };

    let relay_transfer_msg: Ucs03ExecuteMsg = Ucs03ExecuteMsg::Send {
        channel_id: ChannelId::from_raw(channel_id).unwrap(),
        timeout_height: Uint64::from(0u64),
        timeout_timestamp,
        salt,
        instruction: batch_instruction.abi_encode_params().into(),
    };

    let transfer_relay_msg = to_json_binary(&relay_transfer_msg)?;
    Ok(transfer_relay_msg)
}

pub fn ucs03_call(
    sender: String,
    channel_id: u32,
    time: Timestamp,
    hub_contract: String,
    payload: ZkgmHubMsg,
    salt: H256,
) -> Result<Binary, ContractError> {
    let contract_address = match Bytes::from_str(hub_contract.as_str()) {
        Ok(rec) => rec,
        Err(_) => {
            return Err(ContractError::InvalidAddress {
                kind: "recipient".into(),
                address: hub_contract,
                reason: "address must be in hex and starts with 0x".to_string(),
            })
        }
    };

    let call_instruction = Instruction {
        version: INSTR_VERSION_0,
        opcode: OP_CALL,
        operand: Call {
            sender: sender.as_bytes().to_vec().into(),
            eureka: false,
            contract_address,
            contract_calldata: payload.abi_encode_params().into(),
        }
        .abi_encode_params()
        .into(),
    };

    let timeout_timestamp_offset: u64 = 86400; // 1 day period
    let timeout_timestamp =
        Timestamp::from_nanos(time.plus_seconds(timeout_timestamp_offset).nanos());

    let ucs03_send_msg: Ucs03ExecuteMsg = Ucs03ExecuteMsg::Send {
        channel_id: ChannelId::from_raw(channel_id).unwrap(),
        timeout_height: Uint64::from(0u64),
        timeout_timestamp,
        salt,
        instruction: call_instruction.abi_encode_params().into(),
    };

    let ucc03_msg_bin = to_json_binary(&ucs03_send_msg)?;
    Ok(ucc03_msg_bin)
}

pub fn get_hub_ack_msg(
    sender: String,
    ucs03_contract: String,
    channel_id: u32,
    time: Timestamp,
    hub_contract: String,
    payload: ZkgmHubMsg,
    salt: String,
) -> Result<CosmosMsg, ContractError> {
    let salt: unionlabs_primitives::H256 = match unionlabs_primitives::H256::from_str(salt.as_str())
    {
        Ok(s) => s,
        Err(e) => {
            return Err(ContractError::Std(StdError::generic_err(format!(
                "failed to parse salt: {}, reason: {}",
                salt,
                e.to_string()
            ))))
        }
    };

    let msg_bin = ucs03_call(sender, channel_id, time, hub_contract, payload, salt)?;

    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: ucs03_contract.clone(),
        msg: msg_bin,
        funds: vec![],
    });

    Ok(msg)
}
