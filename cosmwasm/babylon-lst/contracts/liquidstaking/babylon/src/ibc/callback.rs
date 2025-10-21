use cosmwasm_std::{
    DepsMut, Env, IbcBasicResponse, IbcDestinationCallbackMsg, StdAck, StdError, StdResult,
    Uint128, ensure_eq, entry_point, from_json,
};
use ibc::apps::transfer::types::proto::transfer::v2::FungibleTokenPacketData;

use crate::{
    event::IbcCallbackEvent,
    state::{PARAMETERS, STATUS, VALIDATORS_REGISTRY},
    utils::{self, transfer::ibc_transfer_msg},
};

fn validate_and_parse_ibc_callback_msg(
    deps: &DepsMut,
    env: &Env,
    msg: &IbcDestinationCallbackMsg,
) -> StdResult<FungibleTokenPacketData> {
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

    let status = STATUS.load(deps.storage)?;
    if status.bond_is_paused {
        return Err(StdError::generic_err(
            "can not bond to this contract while bond is paused",
        ));
    }

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

    Ok(packet_data)
}

fn validate_ibc_denom(
    msg: &IbcDestinationCallbackMsg,
    underlying_denom: &str,
    ibc_packet_denom: &str,
) -> StdResult<()> {
    // We only care about this chain's native token in this example.
    // The `packet_data.denom` is formatted as `{port id}/{channel id}/{denom}`,
    // where the port id and channel id are the source chain's identifiers.
    let native_denom_on_source_chain = format!(
        "{}/{}/{}",
        msg.packet.src.port_id, msg.packet.src.channel_id, underlying_denom,
    );

    ensure_eq!(
        ibc_packet_denom,
        native_denom_on_source_chain,
        StdError::generic_err("unsupported coin denom")
    );

    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
#[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
pub fn ibc_destination_callback(
    deps: DepsMut,
    env: Env,
    msg: IbcDestinationCallbackMsg,
) -> StdResult<IbcBasicResponse> {
    let packet_data = validate_and_parse_ibc_callback_msg(&deps, &env, &msg)?;

    let params = PARAMETERS.load(deps.storage)?;
    validate_ibc_denom(&msg, &params.underlying_coin_denom, &packet_data.denom)?;

    let payload: Result<crate::msg::IBCCallbackPayload, StdError> =
        from_json(packet_data.memo.as_bytes());

    let channel_id: String = msg.packet.dest.channel_id;
    let coin_denom = params.underlying_coin_denom.clone();
    let amount = packet_data
        .amount
        .parse::<u128>()
        .map(Uint128::new)
        .map_err(|_| StdError::generic_err("failed to parse amount as u128"))?;

    let payload = match payload {
        Ok(payload) => payload,
        Err(err) => {
            return Ok(failure_handler(
                env,
                None,
                packet_data.clone(),
                channel_id,
                amount,
                coin_denom,
                err.to_string(),
            ));
        }
    };

    let mut required_amount = payload.amount;

    // if payload transfer fee is set, use it, otherwise use params.transfer_fee
    let transfer_fee = match payload.transfer_fee {
        Some(fee) => fee,
        None => params.transfer_fee,
    };

    // if recipient on other chain, need to add transfer fee to required amount
    if payload.recipient_channel_id.is_some() {
        required_amount += transfer_fee;
    }

    if amount < required_amount {
        let ibc_callback_error_message = format!(
            "insufficient amount, not enough transfer fee, required: {required_amount}, received: {amount}"
        );

        return Ok(failure_handler(
            env,
            Some(payload),
            packet_data,
            channel_id,
            amount,
            coin_denom,
            ibc_callback_error_message,
        ));
    }

    let delegator = env.contract.address.clone();
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;

    let on_chain_recipient = utils::validation::validate_recipient(
        &deps,
        Some(payload.recipient.clone()),
        payload.recipient_channel_id,
        None,
        &Some(payload.salt.clone()),
    );

    if on_chain_recipient.is_err() {
        let ibc_callback_error_message = format!(
            "invalid recipient, reason: {}",
            on_chain_recipient.unwrap_err()
        );
        return Ok(failure_handler(
            env,
            Some(payload),
            packet_data,
            channel_id,
            amount,
            coin_denom,
            ibc_callback_error_message,
        ));
    }

    // // create bond event here
    // let bond_event = crate::event::BondEvent(
    //     packet_data.sender.clone(),
    //     packet_data.sender.clone(),
    //     payload.amount,
    //     bond_data.delegated_amount,
    //     bond_data.mint_amount,
    //     bond_data.total_bond_amount,
    //     bond_data.total_supply,
    //     bond_data.exchange_rate,
    //     "0".into(),
    //     env.block.time,
    //     coin_denom,
    //     Some(payload.recipient.clone()),
    //     payload.recipient_channel_id,
    //     bond_data.reward_balance,
    //     bond_data.unclaimed_reward,
    //     Some(channel_id.clone()),
    // );

    let ibc_callback_event = IbcCallbackEvent(
        packet_data.sender.clone(),
        channel_id.clone(),
        amount,
        payload.amount,
        payload.recipient.clone(),
        payload.recipient_channel_id,
        payload.salt.clone(),
        true,
        String::new(),
        env.block.time,
        transfer_fee,
    );

    Ok(IbcBasicResponse::new())
    // .add_event(ibc_callback_event)
    // .add_event(bond_event)
    // .add_messages(msgs)
    // .add_submessages(submsgs))
}

#[allow(clippy::needless_pass_by_value)]
fn failure_handler(
    env: Env,
    payload: Option<crate::msg::IBCCallbackPayload>,
    packet_data: FungibleTokenPacketData,
    channel_id: String,
    transfer_amount: Uint128,
    denom: String,
    error_message: String,
) -> IbcBasicResponse {
    let msg = ibc_transfer_msg(
        channel_id.clone(),
        packet_data.sender.clone(),
        transfer_amount,
        &denom,
        env.block.time,
    );

    let amount = match payload {
        Some(ref p) => p.amount,
        None => Uint128::zero(),
    };

    let payload_recipient = match payload {
        Some(ref p) => p.recipient.clone(),
        None => String::new(),
    };

    let recipient_channel_id = match payload {
        Some(ref p) => p.recipient_channel_id,
        None => None,
    };

    let salt = match payload {
        Some(ref p) => p.salt.clone(),
        None => String::new(),
    };

    let transfer_fee = match payload {
        Some(ref p) => p.transfer_fee.unwrap_or(Uint128::zero()),
        None => Uint128::zero(),
    };

    let ibc_callback_event = IbcCallbackEvent(
        packet_data.sender.clone(),
        channel_id.clone(),
        transfer_amount,
        amount,
        payload_recipient,
        recipient_channel_id,
        salt,
        false,
        error_message,
        env.block.time,
        transfer_fee,
    );

    IbcBasicResponse::new()
        .add_event(ibc_callback_event)
        .add_message(msg)
}
