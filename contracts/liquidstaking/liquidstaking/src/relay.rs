use crate::msg::Ucs03RelayExecuteMsg;
use crate::token_factory_api::TokenFactoryMsg;
use cosmwasm_std::{to_json_binary, Coin, CosmosMsg, StdError, Uint128, Uint256, WasmMsg};
use unionlabs_primitives::{Bytes, H256};

pub fn send_to_evm(
    contract_addr: String,
    channel_id: u32,
    receiver: Bytes,
    base_token: String,
    base_amount: Uint128,
    quote_token: Bytes,
    quote_amount: Uint256,
    funds: Vec<Coin>,
    salt: H256,
    timeout: u64,
) -> Result<CosmosMsg<TokenFactoryMsg>, StdError> {
    let relay_transfer_msg: Ucs03RelayExecuteMsg = Ucs03RelayExecuteMsg::Transfer {
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

    let msg: CosmosMsg<TokenFactoryMsg> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg: transfer_relay_msg,
        funds,
    });

    Ok(msg)
}
