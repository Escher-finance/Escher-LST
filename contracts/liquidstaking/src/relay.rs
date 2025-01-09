use crate::msg::{UCS03TransferMsg, Ucs03RelayExecuteMsg};
use crate::token_factory_api::TokenFactoryMsg;
use cosmwasm_std::{to_json_binary, Coin, CosmosMsg, StdError, WasmMsg};

pub fn send_to_evm(
    contract_addr: String,
    channel: String,
    receiver: String,
    funds: Vec<Coin>,
    salt: String,
) -> Result<CosmosMsg<TokenFactoryMsg>, StdError> {
    let relay_transfer_msg: Ucs03RelayExecuteMsg = Ucs03RelayExecuteMsg::Transfer(UCS03TransferMsg {
        channel,
        receiver,
        timeout: None,
        salt,
        only_maker: false,
    });
    let transfer_relay_msg = to_json_binary(&relay_transfer_msg)?;

    let msg: CosmosMsg<TokenFactoryMsg> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg: transfer_relay_msg,
        funds,
    });

    Ok(msg)
}
