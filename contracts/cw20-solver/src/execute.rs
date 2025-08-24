use cosmwasm_std::{Addr, DepsMut, Env, Response, Uint256};
use ibc_union_spec::{ChannelId, Packet};
use ucs03_zkgm::com::CwTokenOrderV2;
use unionlabs_primitives::Bytes;

use crate::{
    state::{FungibleLane, FUNGIBLE_COUNTERPARTY},
    ContractError,
};

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
    sender: Addr,
    deps: DepsMut,
    path: Uint256,
    channel_id: ChannelId,
    base_token: Bytes,
    counterparty_beneficiary: Bytes,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &sender)?;
    let key = (path.to_string(), channel_id.raw(), base_token.to_string());
    FUNGIBLE_COUNTERPARTY.save(
        deps.storage,
        key,
        &FungibleLane {
            counterparty_beneficiary,
        },
    )?;
    Ok(Response::new())
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
