use crate::msg::Ucs03ExecuteMsg;
use crate::types::ChannelId;
use crate::ContractError;
use alloy::primitives::U256;
use alloy::sol_types::SolValue;
use cosmwasm_std::{to_json_binary, Binary, Timestamp, Uint128, Uint64};
use unionlabs_primitives::{Bytes, H256};

use super::com::{
    Batch, FungibleAssetOrder, Instruction, INSTR_VERSION_0, INSTR_VERSION_1, OP_BATCH,
    OP_FUNGIBLE_ASSET_ORDER,
};

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
        "eU"
    } else {
        "muno"
    };

    let base_token_name = if base_token == cw20_contract {
        "emuno"
    } else {
        "muno"
    };

    let base_token_path = U256::ZERO;
    let fungible_order_instruction = Instruction {
        version: INSTR_VERSION_1,
        opcode: OP_FUNGIBLE_ASSET_ORDER,
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
