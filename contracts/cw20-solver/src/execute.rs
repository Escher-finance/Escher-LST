use cosmwasm_std::{Addr, DepsMut, Env, Event, Response, Uint256};
use ibc_union_spec::{ChannelId, Packet};
use ucs03_zkgm::{
    com::CwTokenOrderV2,
    contract::{SOLVER_EVENT, SOLVER_EVENT_MARKET_MAKER_ATTR},
};
use unionlabs_primitives::{encoding::HexPrefixed, Bytes};

use crate::{
    helpers::_mint,
    state::{FungibleLane, FUNGIBLE_COUNTERPARTY, ZKGM},
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
    sender: Addr,
    mut deps: DepsMut,
    packet: Packet,
    order: Box<CwTokenOrderV2>,
    path: Uint256,
    _caller: Addr,
    relayer: Addr,
    _relayer_msg: Bytes,
    intent: bool,
) -> Result<Response, ContractError> {
    if intent {
        return Err(ContractError::OnlyFinalized {});
    }
    if sender != ZKGM.load(deps.storage)? {
        return Err(ContractError::OnlyZkgm {});
    }

    let key = (
        path.to_string(),
        packet.destination_channel_id.raw(),
        order.base_token.to_string(),
    );
    let Ok(fungible_lane) = FUNGIBLE_COUNTERPARTY.load(deps.storage, key.clone()) else {
        return Err(ContractError::LaneNotFungible { channel_id: key.1 });
    };

    let mut fee_receiver = None;
    let mut fee_amount = None;

    let fee = order.base_amount.saturating_sub(order.quote_amount);
    if fee > Uint256::zero() {
        fee_receiver = Some(relayer.to_string());
        fee_amount = Some(fee);
        _mint(
            deps.branch(),
            relayer.to_string(),
            fee.try_into().expect("impossible"),
        )?;
    }

    let mut quote_receiver = None;
    let mut quote_amount = None;

    if order.quote_amount > Uint256::zero() {
        let receiver = deps
            .api
            .addr_validate(
                str::from_utf8(order.receiver.as_ref())
                    .map_err(|_| ContractError::InvalidReceiver {})?,
            )
            .map_err(|_| ContractError::InvalidReceiver {})?;
        quote_receiver = Some(receiver.to_string());
        quote_amount = Some(order.quote_amount);
        _mint(
            deps,
            receiver.to_string(),
            order.quote_amount.try_into().expect("impossible"),
        )?;
    }

    Ok(Response::new()
        .add_attribute("action", "do_solve")
        .add_attribute("receiver", quote_receiver.unwrap_or_default())
        .add_attribute("amount", quote_amount.unwrap_or_default())
        .add_attribute("fee_receiver", fee_receiver.unwrap_or_default())
        .add_attribute("fee_amount", fee_amount.unwrap_or_default())
        .add_event(Event::new(SOLVER_EVENT).add_attribute(
            SOLVER_EVENT_MARKET_MAKER_ATTR,
            Bytes::<HexPrefixed>::from(fungible_lane.counterparty_beneficiary.to_vec()).to_string(),
        )))
}
