use std::str::FromStr;

use alloy::{primitives::U256, sol_types::SolValue};
use cosmwasm_std::{
    to_json_binary, wasm_execute, Binary, Coin, CosmosMsg, Timestamp, Uint128, Uint64,
};
use ucs03_zkgm::com::{
    Batch, Instruction, TokenOrderV2, INSTR_VERSION_0, INSTR_VERSION_1, INSTR_VERSION_2, OP_BATCH,
    OP_TOKEN_ORDER, TOKEN_ORDER_KIND_ESCROW,
};
use unionlabs_primitives::{Bytes, H256};

use crate::{
    msg::Ucs03ExecuteMsg,
    state::{PARAMETERS, QUOTE_TOKEN},
    types::ChannelId,
    zkgm::com::FungibleAssetOrder,
    ContractError,
};

#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
pub fn ucs03_transfer(
    cw20_contract: String,
    time: Timestamp,
    channel_id: u32,
    sender: String,
    receiver: Bytes,
    base_token: String,
    base_amount: Uint128,
    quote_token: Bytes,
    quote_amount: Uint128,
    salt: H256,
) -> Result<Binary, ContractError> {
    let base_token_decimals = 6;

    let base_token_symbol = if base_token == cw20_contract {
        "eBABY"
    } else {
        "ubbn"
    };

    let base_token_name = if base_token == cw20_contract {
        "ebbn"
    } else {
        "ubbn"
    };

    let base_token_path = U256::ZERO;
    let fungible_order_instruction = Instruction {
        version: INSTR_VERSION_1,
        opcode: OP_TOKEN_ORDER,
        operand: FungibleAssetOrder {
            sender: sender.as_bytes().to_vec().into(),
            receiver: Vec::from(receiver).into(),
            base_token: base_token.as_bytes().to_vec().into(),
            base_amount: base_amount.u128().try_into().expect("u256>u128"),
            base_token_symbol: base_token_symbol.to_string(),
            base_token_name: base_token_name.to_string(),
            base_token_decimals,
            base_token_path,
            quote_token: Vec::from(quote_token).into(),
            quote_amount: U256::from(quote_amount.u128()),
        }
        .abi_encode_params()
        .into(),
    };

    let batch_instruction = Instruction {
        version: INSTR_VERSION_0,
        opcode: OP_BATCH,
        operand: Batch {
            instructions: vec![fungible_order_instruction],
        }
        .abi_encode_params()
        .into(),
    };

    let timeout_timestamp_offset: u64 = 86400; // 1 day period
    let timeout_timestamp =
        Timestamp::from_nanos(time.plus_seconds(timeout_timestamp_offset).nanos());

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

/// Transfer tokens via UCS03 protocol version 2
/// # Result
/// Will return result of `cosmwasm_std::CosmosMsg`
/// # Errors
/// Will return contract error
#[allow(clippy::needless_pass_by_value)]
pub fn ucs03_transfer_v2(
    deps: cosmwasm_std::DepsMut,
    env: cosmwasm_std::Env,
    sender: &str,
    recipient_address: Bytes,
    amount: Uint128,
    channel_id: ChannelId,
    salt: H256,
) -> Result<CosmosMsg, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;

    let Ok(transfer_amount) = amount.u128().try_into() else {
        return Err(ContractError::InvalidPayload {});
    };

    let quote_token = QUOTE_TOKEN
        .load(deps.storage, channel_id.raw())?
        .quote_token;

    let quote_token: Bytes = match Bytes::from_str(quote_token.as_str()) {
        Ok(rec) => rec,
        Err(_) => {
            return Err(ContractError::InvalidAddress {
                kind: "quote_token".into(),
                address: quote_token,
                reason: "must be in hex and starts with 0x".to_string(),
            });
        }
    };
    let fungible_order_instruction = Instruction {
        version: INSTR_VERSION_2,
        opcode: OP_TOKEN_ORDER,
        operand: TokenOrderV2 {
            sender: sender.as_bytes().to_vec().into(),
            receiver: Vec::from(recipient_address.clone()).into(),
            base_token: params
                .underlying_coin_denom
                .clone()
                .as_bytes()
                .to_vec()
                .into(),
            base_amount: transfer_amount,
            quote_token: Vec::from(quote_token).into(),
            quote_amount: transfer_amount,
            kind: TOKEN_ORDER_KIND_ESCROW,
            metadata: "".as_bytes().to_vec().into(),
        }
        .abi_encode_params()
        .into(),
    };

    let timeout_timestamp_offset: u64 = 86400; // 1 day period
    let timeout_timestamp = Timestamp::from_nanos(
        env.block
            .time
            .plus_seconds(timeout_timestamp_offset)
            .nanos(),
    );

    let transfer_msg: Ucs03ExecuteMsg = Ucs03ExecuteMsg::Send {
        channel_id,
        timeout_height: Uint64::from(0u64),
        timeout_timestamp,
        salt,
        instruction: fungible_order_instruction.abi_encode_params().into(),
    };

    let funds = vec![Coin {
        denom: params.underlying_coin_denom.clone(),
        amount,
    }];
    let msg = wasm_execute(params.ucs03_relay_contract.clone(), &transfer_msg, funds)?;

    Ok(msg.into())
}
