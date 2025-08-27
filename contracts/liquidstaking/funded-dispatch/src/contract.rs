use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    entry_point, from_json, wasm_execute, Addr, Coin, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint256,
};
use cw_storage_plus::Item;
use ibc_union_spec::ChannelId;
use serde::{Deserialize, Serialize};
use unionlabs_primitives::Bytes;

pub const LST_ADDRESS: Item<Addr> = Item::new("lst");

#[cw_serde]
pub struct InstantiateMsg {
    pub lst_address: Addr,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
    LST_ADDRESS.save(deps.storage, &msg.lst_address)?;

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
    message: Bytes,
    funds: Vec<Coin>,
}

pub fn on_zkgm(deps: Deps, message: Bytes) -> StdResult<Response> {
    let message: Message = from_json(message)?;
    Ok(Response::new().add_message(wasm_execute(
        LST_ADDRESS.load(deps.storage)?,
        &message.message,
        message.funds,
    )?))
}
