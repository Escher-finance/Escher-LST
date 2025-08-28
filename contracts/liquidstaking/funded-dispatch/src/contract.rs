use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint256, entry_point,
    from_json, wasm_execute,
};
use cw_storage_plus::Item;
use ibc_union_spec::ChannelId;
use serde::{Deserialize, Serialize};
use unionlabs_primitives::Bytes;

#[cw_serde]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ExecuteMsg {
    OnZkgm {
        caller: Addr,
        path: Uint256,
        source_channel_id: ChannelId,
        destination_channel_id: ChannelId,
        sender: Bytes,
        message: Bytes,
        relayer: Addr,
        relayer_msg: Bytes,
    },
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _: Env,
    _: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::new())
}

#[entry_point]
pub fn execute(deps: DepsMut, _: Env, _: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::OnZkgm { message, .. } => on_zkgm(deps.as_ref(), message),
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Message {
    address: Addr,
    message: Bytes,
    funds: Vec<Coin>,
}

pub fn on_zkgm(deps: Deps, message: Bytes) -> StdResult<Response> {
    let message: Message = from_json(message)?;
    Ok(Response::new().add_message(wasm_execute(
        message.address,
        &message.message,
        message.funds,
    )?))
}
