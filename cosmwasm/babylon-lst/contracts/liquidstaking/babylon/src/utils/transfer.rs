use std::str::FromStr;

use alloy::sol_types::SolValue;
use cosmwasm_std::{
    Addr, BankMsg, Coin, CosmosMsg, IbcMsg, IbcTimeout, StdError, Storage, Timestamp, Uint64,
    Uint128, wasm_execute,
};
use ucs03_zkgm::com::{
    INSTR_VERSION_2, Instruction, OP_TOKEN_ORDER, TOKEN_ORDER_KIND_ESCROW, TokenOrderV2,
};
use unionlabs_primitives::Bytes;

use crate::{
    ContractError,
    execute::StakerUndelegation,
    msg::{Ucs03ExecuteMsg, ZkgmTransfer},
    types::ChannelId,
    utils::delegation::{
        DEFAULT_TIMEOUT_TIMESTAMP_OFFSET, get_unbonding_ucs03_transfer_cosmos_msg,
    },
};

#[must_use]
pub fn get_send_bank_msg(
    staker: &str,
    recipient: Option<&String>,
    denom: &str,
    amount: Uint128,
) -> CosmosMsg {
    let recipient = match recipient {
        Some(addr) => addr.clone(),
        None => staker.to_owned(),
    };
    let bank_msg = BankMsg::Send {
        to_address: recipient,
        amount: vec![Coin {
            denom: denom.to_string(),
            amount,
        }],
    };
    CosmosMsg::Bank(bank_msg)
}

#[allow(clippy::too_many_arguments)]
pub fn send_back_token_via_ucs03(
    storage: &mut dyn Storage,
    lst_contract: &Addr,
    staker: &str,
    denom: &str,
    transfer_handler: &str,
    transfer_fee: Uint128,
    ucs03_relay_contract: &str,
    undelegation: &StakerUndelegation,
    time: Timestamp,
    salt: &str,
) -> Result<(CosmosMsg, CosmosMsg), ContractError> {
    let bank_msg = BankMsg::Send {
        to_address: transfer_handler.to_string(),
        amount: vec![Coin {
            denom: denom.to_string(),
            amount: undelegation.unstake_return_native_amount.unwrap(),
        }],
    };
    let bank_msg: CosmosMsg = CosmosMsg::Bank(bank_msg);

    let target_channel_id = match undelegation.recipient_channel_id {
        Some(ch_id) => ch_id,
        None => undelegation.channel_id.unwrap(),
    };

    let receiver = match undelegation.recipient.clone() {
        Some(rec) => rec,
        None => staker.to_owned(),
    };
    //after send bank msg to transfer handler, then call ucs03 on behalf of transfer handler to send token back
    let ucs3_send_msg = get_unbonding_ucs03_transfer_cosmos_msg(
        storage,
        lst_contract,
        receiver,
        target_channel_id,
        time,
        ucs03_relay_contract,
        undelegation.unstake_return_native_amount.unwrap(),
        transfer_fee,
        denom,
        salt,
    )?;
    Ok((bank_msg, ucs3_send_msg))
}

#[must_use]
pub fn ibc_transfer_msg(
    channel_id: String,
    to_address: String,
    transfer_amount: Uint128,
    denom: &str,
    block_time: Timestamp,
) -> CosmosMsg {
    let timeout = block_time.plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET);
    // send native token back via ibc
    let amount = Coin {
        amount: transfer_amount,
        denom: denom.to_string(),
    };
    CosmosMsg::Ibc(IbcMsg::Transfer {
        channel_id,
        to_address,
        amount,
        timeout: IbcTimeout::with_timestamp(timeout),
        memo: None,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn send_token_order_v2_escrow(
    ucs03_relay_contract: &str,
    payload: &ZkgmTransfer,
    base_token: &str,
    quote_token: &str,
) -> Result<CosmosMsg, ContractError> {
    let Ok(transfer_amount) = payload.amount.u128().try_into() else {
        return Err(ContractError::InvalidPayload {});
    };
    let receiver: Bytes = match Bytes::from_str(payload.recipient.as_str()) {
        Ok(r) => r,
        Err(_) => {
            return Err(ContractError::InvalidAddress {
                kind: "recipient".into(),
                address: payload.recipient.clone(),
                reason: "address must be in hex and starts with 0x".to_string(),
            });
        }
    };
    let quote_token: Bytes = match Bytes::from_str(quote_token) {
        Ok(q) => q,
        Err(_) => {
            return Err(ContractError::InvalidAddress {
                kind: "quote_token".into(),
                address: quote_token.to_string(),
                reason: "address must be in hex and starts with 0x".to_string(),
            });
        }
    };

    let fungible_order_instruction = Instruction {
        version: INSTR_VERSION_2,
        opcode: OP_TOKEN_ORDER,
        operand: TokenOrderV2 {
            sender: payload.sender.as_bytes().to_vec().into(),
            receiver: Vec::from(receiver).into(),
            base_token: base_token.as_bytes().to_vec().into(),
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
    let timeout_timestamp =
        Timestamp::from_nanos(payload.time.plus_seconds(timeout_timestamp_offset).nanos());

    let salt: unionlabs_primitives::H256 =
        match unionlabs_primitives::H256::from_str(payload.salt.as_str()) {
            Ok(s) => s,
            Err(e) => {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "failed to parse salt: {0}, reason: {e}",
                    payload.salt
                ))));
            }
        };

    let Some(channel_id) = ChannelId::from_raw(payload.recipient_channel_id) else {
        return Err(ContractError::InvalidChannelId {});
    };

    let send_msg: Ucs03ExecuteMsg = Ucs03ExecuteMsg::Send {
        channel_id,
        timeout_height: Uint64::from(0u64),
        timeout_timestamp,
        salt,
        instruction: fungible_order_instruction.abi_encode_params().into(),
    };

    let funds = vec![Coin {
        denom: base_token.to_string(),
        amount: payload.amount,
    }];
    let msg = wasm_execute(ucs03_relay_contract, &send_msg, funds)?;

    Ok(msg.into())
}
