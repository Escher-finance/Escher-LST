use cosmwasm_std::{
    ensure_eq, entry_point, from_json, DepsMut, Env, IbcBasicResponse, IbcDestinationCallbackMsg,
    StdAck, StdError, StdResult, Uint128,
};
use ibc::apps::transfer::types::proto::transfer::v2::FungibleTokenPacketData;

use crate::{
    state::{PARAMETERS, VALIDATORS_REGISTRY},
    utils::delegation::process_bond,
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
    // Please note that the denom is not formatted like that when you transfer untrn
    // from Neutron to some other chain. In that case, the denom is just the native
    // token of the source chain (ubbn).
    let native_denom_on_source_chain = format!(
        "{}/{}/{}",
        msg.packet.src.port_id, msg.packet.src.channel_id, params.underlying_coin_denom
    );

    let memo = packet_data.memo;
    let payload: Result<crate::msg::IBCCallbackPayload, StdError> = from_json(memo.as_bytes());

    if payload.is_ok() {
        let payload = payload.unwrap();
        let amount = packet_data
            .amount
            .parse::<u128>()
            .map(Uint128::new)
            .map_err(|_| StdError::generic_err("failed to parse amount as u128"))?;
        let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
        let channel_id = msg.packet.dest.channel_id.parse::<u32>().map_err(|_| {
            StdError::generic_err(format!(
                "Failed to parse '{}' as u32",
                msg.packet.dest.channel_id
            ))
        })?;

        let salt = payload.salt;

        // if the denom is native token then do the bond
        if packet_data.denom == native_denom_on_source_chain {
            let (_msgs, _sub_msgs, _bond_data) = process_bond(
                deps.storage,
                deps.querier,
                env.contract.address.to_string(),
                packet_data.sender,
                env.contract.address,
                amount,
                env.block.time.nanos(),
                params,
                validators_reg,
                salt,
                Some(channel_id),
                env.block.height,
            )
            .map_err(|err| StdError::generic_err(format!("process_bond failed: {}", err)))?;
        }

        // TODO
        // handle unbond
        // process msgs and submsgs
    }

    Ok(IbcBasicResponse::new().add_attribute("action", "ibc_destination_callback"))
}
