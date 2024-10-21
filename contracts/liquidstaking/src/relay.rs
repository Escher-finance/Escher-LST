use crate::msg::{TransferMsg, Ucs01RelayExecuteMsg};
use crate::token_factory_api::TokenFactoryMsg;
use cosmwasm_std::{to_json_binary, Coin, CosmosMsg, StdError, WasmMsg};

pub fn send_to_evm(
    contract_addr: String,
    channel: String,
    receiver: String,
    funds: Vec<Coin>,
) -> Result<CosmosMsg<TokenFactoryMsg>, StdError> {
    let relay_transfer_msg: Ucs01RelayExecuteMsg = Ucs01RelayExecuteMsg::Transfer(TransferMsg {
        channel,
        receiver,
        memo: "Send back to EVM".to_string(),
        timeout: None,
    });
    let transfer_relay_msg = to_json_binary(&relay_transfer_msg)?;

    let msg: CosmosMsg<TokenFactoryMsg> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg: transfer_relay_msg,
        funds,
    });

    Ok(msg)
}
