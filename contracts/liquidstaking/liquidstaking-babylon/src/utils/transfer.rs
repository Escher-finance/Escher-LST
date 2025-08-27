use cosmwasm_std::{
    Addr, BankMsg, Coin, CosmosMsg, IbcMsg, IbcTimeout, Storage, Timestamp, Uint128,
};

use crate::{
    ContractError,
    execute::StakerUndelegation,
    utils::delegation::{
        DEFAULT_TIMEOUT_TIMESTAMP_OFFSET, get_unbonding_ucs03_transfer_cosmos_msg,
    },
};

pub fn get_send_bank_msg(
    staker: &str,
    recipient: Option<String>,
    denom: String,
    amount: Uint128,
) -> CosmosMsg {
    let recipient = match recipient.clone() {
        Some(addr) => addr,
        None => staker.to_owned(),
    };
    let bank_msg = BankMsg::Send {
        to_address: recipient,
        amount: vec![Coin { denom, amount }],
    };
    CosmosMsg::Bank(bank_msg)
}

#[allow(clippy::too_many_arguments)]
pub fn send_back_token_via_ucs03(
    storage: &mut dyn Storage,
    lst_contract: Addr,
    staker: &str,
    denom: String,
    transfer_handler: String,
    transfer_fee: Uint128,
    ucs03_relay_contract: String,
    undelegation: &StakerUndelegation,
    time: Timestamp,
    salt: String,
) -> Result<(CosmosMsg, CosmosMsg), ContractError> {
    let bank_msg = BankMsg::Send {
        to_address: transfer_handler.clone(),
        amount: vec![Coin {
            denom: denom.clone(),
            amount: undelegation.unstake_return_native_amount.unwrap(),
        }],
    };
    let bank_msg: CosmosMsg = CosmosMsg::Bank(bank_msg);

    let target_channel_id = match undelegation.recipient_channel_id {
        Some(ch_id) => ch_id,
        None => undelegation.channel_id.unwrap(),
    };

    let receiver = match undelegation.recipient.clone() {
        Some(rec) => rec,
        None => staker.to_owned(),
    };
    //after send bank msg to transfer handler, then call ucs03 on behalf of transfer handler to send token back
    let ucs3_send_msg = get_unbonding_ucs03_transfer_cosmos_msg(
        storage,
        lst_contract.clone(),
        receiver,
        target_channel_id,
        time,
        ucs03_relay_contract.clone(),
        undelegation.unstake_return_native_amount.unwrap(),
        transfer_fee,
        denom,
        salt,
    )?;
    Ok((bank_msg, ucs3_send_msg))
}

pub fn ibc_transfer_msg(
    channel_id: String,
    to_address: String,
    transfer_amount: Uint128,
    denom: String,
    block_time: Timestamp,
) -> CosmosMsg {
    let timeout = block_time.plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET);
    // send native token back via ibc
    let amount = Coin {
        amount: transfer_amount,
        denom: denom.clone(),
    };
    CosmosMsg::Ibc(IbcMsg::Transfer {
        channel_id,
        to_address,
        amount,
        timeout: IbcTimeout::with_timestamp(timeout),
        memo: None,
    })
}
