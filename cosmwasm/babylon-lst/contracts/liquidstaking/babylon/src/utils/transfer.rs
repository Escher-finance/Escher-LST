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

#[must_use]
pub fn get_send_bank_msg(
    staker: &str,
    recipient: Option<&String>,
    denom: &str,
    amount: Uint128,
) -> CosmosMsg {
    let recipient = match recipient {
        Some(addr) => addr.clone(),
        None => staker.to_owned(),
    };
    let bank_msg = BankMsg::Send {
        to_address: recipient,
        amount: vec![Coin { denom: denom.to_string(), amount }],
    };
    CosmosMsg::Bank(bank_msg)
}

#[allow(clippy::too_many_arguments)]
pub fn send_back_token_via_ucs03(
    storage: &mut dyn Storage,
    lst_contract: &Addr,
    staker: &str,
    denom: &str,
    transfer_handler: &str,
    transfer_fee: Uint128,
    ucs03_relay_contract: &str,
    undelegation: &StakerUndelegation,
    time: Timestamp,
    salt: &str,
) -> Result<(CosmosMsg, CosmosMsg), ContractError> {
    let bank_msg = BankMsg::Send {
        to_address: transfer_handler.to_string(),
        amount: vec![Coin {
            denom: denom.to_string(),
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
        lst_contract,
        receiver,
        target_channel_id,
        time,
        ucs03_relay_contract,
        undelegation.unstake_return_native_amount.unwrap(),
        transfer_fee,
        denom,
        salt,
    )?;
    Ok((bank_msg, ucs3_send_msg))
}

#[must_use]
pub fn ibc_transfer_msg(
    channel_id: String,
    to_address: String,
    transfer_amount: Uint128,
    denom: &str,
    block_time: Timestamp,
) -> CosmosMsg {
    let timeout = block_time.plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET);
    // send native token back via ibc
    let amount = Coin {
        amount: transfer_amount,
        denom: denom.to_string(),
    };
    CosmosMsg::Ibc(IbcMsg::Transfer {
        channel_id,
        to_address,
        amount,
        timeout: IbcTimeout::with_timestamp(timeout),
        memo: None,
    })
}
