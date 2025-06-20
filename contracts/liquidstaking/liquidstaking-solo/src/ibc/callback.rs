use cosmwasm_std::{
    ensure_eq, entry_point, from_json, DepsMut, Env, IbcBasicResponse, IbcDestinationCallbackMsg,
    StdAck, StdError, StdResult, Uint128,
};
use ibc::apps::transfer::types::proto::transfer::v2::FungibleTokenPacketData;

use crate::{
    state::{PARAMETERS, VALIDATORS_REGISTRY},
    utils,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_destination_callback(
    deps: DepsMut,
    env: Env,
    msg: IbcDestinationCallbackMsg,
) -> StdResult<IbcBasicResponse> {
    ensure_eq!(
        msg.packet.dest.port_id,
        "transfer", // transfer module uses this port by default
        StdError::generic_err("only want to handle transfer packets")
    );
    ensure_eq!(
        msg.ack.data,
        StdAck::success(b"\x01").to_binary(), // this is how a successful transfer ack looks
        StdError::generic_err("only want to handle successful transfers")
    );
    // At this point we know that this is a callback for a successful transfer,
    // but not to whom it is going, how much and what denom.

    // Parse the packet data to get that information:
    let packet_data: FungibleTokenPacketData = from_json(&msg.packet.data)?;

    // The receiver should be a valid address on this chain.
    // Remember, we are on the destination chain.
    let receiver = deps.api.addr_validate(packet_data.receiver.as_ref())?;
    ensure_eq!(
        receiver,
        env.contract.address,
        StdError::generic_err("only want to handle transfers to this contract")
    );

    let params = PARAMETERS.load(deps.storage)?;

    // We only care about this chain's native token in this example.
    // The `packet_data.denom` is formatted as `{port id}/{channel id}/{denom}`,
    // where the port id and channel id are the source chain's identifiers.
    let native_denom_on_source_chain = format!(
        "{}/{}/{}",
        msg.packet.src.port_id, msg.packet.src.channel_id, params.underlying_coin_denom
    );

    ensure_eq!(
        packet_data.denom,
        native_denom_on_source_chain,
        StdError::generic_err("unsupported coin denom")
    );

    let memo = packet_data.memo;
    let payload: Result<crate::msg::IBCCallbackPayload, StdError> = from_json(memo.as_bytes());

    let channel_id: String = msg.packet.dest.channel_id;

    if payload.is_err() {
        return Err(StdError::generic_err("invalid payload"));
    }

    let payload = payload.unwrap();
    let amount = packet_data
        .amount
        .parse::<u128>()
        .map(Uint128::new)
        .map_err(|_| StdError::generic_err("failed to parse amount as u128"))?;

    // check amount

    let mut required_amount = payload.amount;
    // if recipient on other chain, need to add transfer fee to required amount
    if payload.recipient_channel_id.is_some() {
        required_amount += params.transfer_fee;
    }

    if amount < required_amount {
        return Err(StdError::generic_err("not enough fund"));
    }

    let salt = payload.salt;

    let delegator = env.contract.address;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;

    let on_chain_recipient = utils::validation::validate_recipient(
        &deps,
        Some(payload.recipient.clone()),
        payload.recipient_channel_id,
        None,
        Some(salt.clone()),
    );

    if on_chain_recipient.is_err() {
        return Err(StdError::generic_err(format!(
            "invalid recipient, reason: {}",
            on_chain_recipient.unwrap_err().to_string()
        )));
    }

    let coin_denom = params.underlying_coin_denom.clone();
    let process_bond_result = utils::delegation::process_bond(
        deps.storage,
        deps.querier,
        packet_data.sender.clone(),
        packet_data.sender.clone(),
        delegator.clone(),
        payload.amount.clone(),
        env.block.time.nanos(),
        params,
        validators_reg.clone(),
        salt,
        None,
        env.block.height,
        Some(payload.recipient.clone()),
        payload.recipient_channel_id,
        on_chain_recipient.unwrap(),
    );

    if process_bond_result.is_err() {
        return Err(StdError::generic_err(format!(
            "process_bond failed, reason: {}",
            process_bond_result.unwrap_err().to_string()
        )));
    }

    let (msgs, submsgs, bond_data) = process_bond_result.unwrap();

    // create bond event here
    let bond_event = crate::event::BondEvent(
        packet_data.sender.to_string(),
        packet_data.sender.clone(),
        payload.amount.clone(),
        bond_data.delegated_amount.clone(),
        bond_data.mint_amount,
        bond_data.total_bond_amount.clone(),
        bond_data.total_supply,
        bond_data.exchange_rate,
        "0".into(),
        env.block.time,
        coin_denom,
        Some(payload.recipient),
        payload.recipient_channel_id,
        bond_data.reward_balance,
        bond_data.unclaimed_reward,
        Some(channel_id),
    );

    Ok(IbcBasicResponse::new()
        .add_event(bond_event)
        .add_messages(msgs)
        .add_submessages(submsgs))
}
