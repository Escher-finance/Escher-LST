use cosmwasm_std::{Addr, DepsMut, Env, Response, Uint256};
use ibc_union_spec::{ChannelId, Packet};
use ucs03_zkgm::com::CwTokenOrderV2;
use unionlabs_primitives::Bytes;

use crate::ContractError;

pub fn update_ownership(
    deps: DepsMut,
    env: Env,
    new_owner: Addr,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    cw_ownable::update_ownership(deps, &env.block, &new_owner, action)?;
    Ok(Response::new())
}

pub fn set_fungible_counterparty(
    deps: DepsMut,
    path: Uint256,
    channel_id: ChannelId,
    base_token: Bytes,
    counterparty_beneficiary: Bytes,
) -> Result<Response, ContractError> {
    todo!()
}

pub fn do_solve(
    packet: Packet,
    order: Box<CwTokenOrderV2>,
    path: Uint256,
    caller: Addr,
    relayer: Addr,
    relayer_msg: Bytes,
    intent: bool,
) -> Result<Response, ContractError> {
    todo!()
}
