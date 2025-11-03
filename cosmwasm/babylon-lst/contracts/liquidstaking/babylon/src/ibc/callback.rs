use cosmwasm_std::{
    CosmosMsg, DepsMut, Env, IbcBasicResponse, IbcDestinationCallbackMsg, StdAck, StdError,
    StdResult, SubMsg, Uint128, WasmMsg, ensure_eq, entry_point, from_json, to_json_binary,
};
use ibc::apps::transfer::types::proto::transfer::v2::FungibleTokenPacketData;

use crate::{
    event::{BondEvent, BondEventParams, IbcCallbackEvent},
    msg::{Recipient, ZkgmTransfer},
    reply::MINT_AND_SEND_ZKGM_REPLY_ID,
    state::{PARAMETERS, STATUS},
    utils::{
        self,
        transfer::ibc_transfer_msg,
        validation::{split_and_validate_recipient, validate_salt},
    },
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

    // Parse the packet data to get the ibc transfer information:
    let packet_data: FungibleTokenPacketData = from_json(&msg.packet.data)?;

    // The receiver should be this contract address on this chain.
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

    let ibc_channel_id: String = msg.packet.dest.channel_id;
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
                ibc_channel_id,
                amount,
                coin_denom,
                err.to_string(),
            ));
        }
    };

    if amount != payload.amount {
        let ibc_callback_error_message = format!(
            "incorrect amount, required: {}, received: {amount}",
            payload.amount
        );

        return Ok(failure_handler(
            env,
            Some(payload),
            packet_data,
            ibc_channel_id,
            amount,
            coin_denom,
            ibc_callback_error_message,
        ));
    }

    // handle delegation to validators
    let result = utils::delegation::delegate(
        deps.storage,
        deps.querier,
        env.clone(),
        amount,
        payload.min_mint_amount,
        payload.slippage,
    );

    let (mut msgs, bond_data) = match result {
        Ok((msgs, bond_data)) => (msgs, bond_data),
        Err(e) => {
            return Ok(failure_handler(
                env,
                Some(payload.clone()),
                packet_data,
                ibc_channel_id,
                amount,
                coin_denom,
                e.to_string(),
            ));
        }
    };

    let (the_recipient, recipient_channel_id, recipient_ibc_channel_id) =
        match split_and_validate_recipient(
            deps.storage,
            deps.api,
            payload.recipient.clone(),
            &crate::msg::RecipientAction::Bond,
        ) {
            Ok((the_recipient, recipient_channel_id, recipient_ibc_channel_id)) => (
                the_recipient,
                recipient_channel_id,
                recipient_ibc_channel_id,
            ),
            Err(e) => {
                let ibc_callback_error_message = format!("invalid recipient, reason: {e}");
                return Ok(failure_handler(
                    env,
                    Some(payload),
                    packet_data,
                    ibc_channel_id,
                    amount,
                    coin_denom,
                    ibc_callback_error_message,
                ));
            }
        };

    let mut sub_msgs: Vec<SubMsg> = vec![];
    match payload.recipient {
        Recipient::OnChain { address } => {
            // mint staked token to on chain recipient address
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: bond_data.cw20_address.clone(),
                msg: to_json_binary(&cw20::Cw20ExecuteMsg::Mint {
                    recipient: address.to_string(),
                    amount: bond_data.mint_amount,
                })?,
                funds: vec![],
            }));
        }
        Recipient::Zkgm {
            address,
            channel_id,
        } => {
            // mint staked token to this contract
            let mint_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: bond_data.cw20_address.clone(),
                msg: to_json_binary(&cw20::Cw20ExecuteMsg::Mint {
                    recipient: env.contract.address.to_string(),
                    amount: bond_data.mint_amount,
                })?,
                funds: vec![],
            });

            validate_salt(&payload.salt).map_err(|err| StdError::generic_err(err.to_string()))?;

            let payload_bin = to_json_binary(&ZkgmTransfer {
                sender: env.contract.address.to_string(),
                amount: bond_data.mint_amount,
                recipient: address,
                recipient_channel_id: channel_id,
                salt: payload.salt.clone(),
                time: env.block.time,
            })?;
            // create sub msg so we can send via zkgm on reply
            let sub_msg: SubMsg = SubMsg::reply_always(mint_msg, MINT_AND_SEND_ZKGM_REPLY_ID)
                .with_payload(payload_bin);
            sub_msgs.push(sub_msg);
        }
        Recipient::Ibc {
            address: _,
            ibc_channel_id: _,
        } => {
            return Ok(failure_handler(
                env,
                Some(payload),
                packet_data,
                ibc_channel_id,
                amount,
                coin_denom,
                "can not send liquid staking token to ibc recipient".to_string(),
            ));
        }
    }

    let bond_event = BondEvent(BondEventParams {
        sender: packet_data.sender.clone(),
        staker: packet_data.sender.clone(),
        min_mint_amount: payload.min_mint_amount,
        bond_data,
        channel_id: String::new(),
        time: env.block.time,
        recipient: the_recipient.clone(),
        recipient_channel_id,
        ibc_channel_id: recipient_ibc_channel_id,
    });

    let ibc_callback_event = IbcCallbackEvent(
        packet_data.sender,
        ibc_channel_id.clone(),
        amount,
        payload.amount,
        the_recipient.unwrap_or_default(),
        recipient_channel_id,
        payload.salt.clone(),
        true,
        String::new(),
        env.block.time,
    );

    Ok(IbcBasicResponse::new()
        .add_event(ibc_callback_event)
        .add_event(bond_event)
        .add_messages(msgs)
        .add_submessages(sub_msgs))
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
    // create send ibc message to transfer back to sender
    let msg = ibc_transfer_msg(
        channel_id.clone(),
        packet_data.sender.clone(),
        transfer_amount,
        &denom,
        env.block.time,
    );

    let mut amount = Uint128::zero();
    let mut salt = String::new();
    let mut recipient = String::new();
    let mut recipient_channel_id: Option<u32> = None;

    if let Some(p) = payload {
        amount = p.amount;
        salt = p.salt;

        match p.recipient {
            Recipient::OnChain { address } => {
                recipient = address.to_string();
            }
            Recipient::Zkgm {
                address,
                channel_id,
            } => {
                recipient = address;
                recipient_channel_id = Some(channel_id);
            }
            Recipient::Ibc {
                address: _,
                ibc_channel_id: _,
            } => {}
        }
    }

    let ibc_callback_event = IbcCallbackEvent(
        packet_data.sender.clone(),
        channel_id.clone(),
        transfer_amount,
        amount,
        recipient,
        recipient_channel_id,
        salt,
        false,
        error_message,
        env.block.time,
    );

    IbcBasicResponse::new()
        .add_event(ibc_callback_event)
        .add_message(msg)
}
