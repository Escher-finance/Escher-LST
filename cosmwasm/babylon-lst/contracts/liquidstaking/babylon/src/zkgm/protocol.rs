use std::str::FromStr;

use alloy::{primitives::U256, sol_types::SolValue};
use cosmwasm_std::{
    Addr, Binary, Coin, CosmosMsg, Env, Timestamp, Uint64, Uint128, to_json_binary, wasm_execute,
};
use sha2::{Digest, Sha256};
use ucs03_zkgm::com::{
    Batch, INSTR_VERSION_0, INSTR_VERSION_1, INSTR_VERSION_2, Instruction, OP_BATCH,
    OP_TOKEN_ORDER, TOKEN_ORDER_KIND_ESCROW, TokenOrderV2,
};
use unionlabs_primitives::{Bytes, H256};

use crate::{
    ContractError,
    msg::{Ucs03ExecuteMsg, ZkgmTransfer},
    state::{PARAMETERS, QUOTE_TOKEN},
    types::ChannelId,
    utils::transfer::send_token_order_v2_escrow,
    zkgm::com::FungibleAssetOrder,
};

/// Generate a salt based on block timestamp and sender address
/// Equivalent to Solidity:
/// ```solidity
/// bytes memory rawSalt = abi.encodePacked(block.timestamp, msg.sender);
/// bytes32 salt = keccak256(rawSalt);
/// ```
///
/// # Arguments
/// * `env` - Environment containing block information (time)
/// * `sender` - Address of the message sender
///
/// # Returns
/// * `String` - The generated salt as a 0x string
#[must_use]
pub fn generate_salt(env: &Env, sender: &Addr) -> String {
    let mut hasher = Sha256::new();

    // Encode timestamp (block.time in nanoseconds as u64)
    let timestamp_nanos = env.block.time.nanos();
    hasher.update(timestamp_nanos.to_be_bytes());

    // Encode sender address
    hasher.update(sender.as_bytes());

    // Finalize hash and convert to hex string
    format!("0x{:x}", hasher.finalize())
}

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

#[derive(Debug)]
pub struct Ucs03Zkgm {
    pub contract: String,
    pub token_pair: TokenPair,
}

#[derive(Debug)]
pub struct TokenPair {
    pub base_token: String,
    pub quote_token: String,
}

impl Ucs03Zkgm {
    #[must_use]
    pub fn new(contract: String, token_pair: TokenPair) -> Ucs03Zkgm {
        Ucs03Zkgm {
            contract,
            token_pair,
        }
    }

    pub fn transfer_escrow_with_funds(
        &self,
        payload: &ZkgmTransfer,
        funds: &[Coin],
    ) -> Result<CosmosMsg, ContractError> {
        send_token_order_v2_escrow(&self.contract, payload, &self.token_pair, funds)
    }
}
