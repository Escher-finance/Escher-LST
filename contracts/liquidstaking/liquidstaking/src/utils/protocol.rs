use crate::msg::Ucs03ExecuteMsg;
use crate::utils::delegation::DEFAULT_TIMEOUT_TIMESTAMP_OFFSET;
use crate::ContractError;
use cosmwasm_std::{to_json_binary, Coin, Env, Timestamp, Uint128, Uint256, WasmMsg};
use unionlabs_primitives::{Bytes, H256};

pub fn ucs03_transfer(
    time: Timestamp,
    ucs03_contract_addr: String,
    channel_id: u32,
    receiver: Bytes,
    base_token: String,
    base_amount: Uint128,
    quote_token: Bytes,
    quote_amount: Uint256,
    funds: Vec<Coin>,
    salt: H256,
) -> Result<WasmMsg, ContractError> {
    let timeout = time.plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET).nanos();

    let relay_transfer_msg: Ucs03ExecuteMsg = Ucs03ExecuteMsg::Transfer {
        channel_id,
        receiver,
        base_token,
        base_amount,
        quote_token,
        quote_amount,
        timeout_height: 0,
        timeout_timestamp: timeout,
        salt,
    };

    let transfer_relay_msg = to_json_binary(&relay_transfer_msg)?;

    return Ok(WasmMsg::Execute {
        contract_addr: ucs03_contract_addr,
        msg: transfer_relay_msg,
        funds,
    });
}
