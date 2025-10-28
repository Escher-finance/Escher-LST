use std::collections::HashSet;

use cosmwasm_std::{Api, Coin, Storage};

use crate::{
    ContractError,
    msg::Recipient,
    state::{QuoteToken, Validator},
};

/// Errors:
/// - Returns `InvalidValidators` when duplicate addresses or zero weights are found.
pub fn validate_validators(validators: &[Validator]) -> Result<(), ContractError> {
    let unique_validators_len = validators
        .iter()
        .cloned()
        .map(|validator| validator.address)
        .collect::<HashSet<_>>()
        .len();

    if unique_validators_len != validators.len() {
        return Err(ContractError::InvalidValidators {});
    }

    for validator in validators {
        if validator.weight == 0 {
            return Err(ContractError::InvalidValidators {});
        }
    }

    Ok(())
}

/// Errors:
/// - Returns `InvalidQuoteTokens` when duplicate channel ids are found.
pub fn validate_quote_tokens(quote_tokens: &[QuoteToken]) -> Result<(), ContractError> {
    let unique_quote_tokens_len = quote_tokens
        .iter()
        .cloned()
        .map(|quote_token| quote_token.channel_id)
        .collect::<HashSet<_>>()
        .len();

    if unique_quote_tokens_len != quote_tokens.len() {
        return Err(ContractError::InvalidQuoteTokens {});
    }

    Ok(())
}

/// Errors:
/// - Returns address/channel validation errors
pub fn validate_recipient(
    storage: &dyn Storage,
    api: &dyn Api,
    recipient: Option<String>,
    recipient_channel_id: Option<u32>,
    recipient_ibc_channel_id: Option<String>,
) -> Result<bool, ContractError> {
    let mut on_chain_recipient = false;
    // if recipient is provided but channel id is none, need to validate the address as it is the same chain address as contract
    if recipient_channel_id.is_none() && recipient_ibc_channel_id.is_none() {
        match recipient.as_ref() {
            Some(recipient) => api.addr_validate(recipient.as_str())?,
            None => {
                return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                    "missing recipient",
                )));
            }
        };
        on_chain_recipient = true;
    }

    // if recipient_channel_id exists, must make sure the chain is supported and recipient address is in hex
    if let Some(recipient_channel_id) = recipient_channel_id {
        if recipient_channel_id == 0 {
            return Err(ContractError::InvalidChannelId {});
        }
        let channel_id = crate::state::CHAINS.load(storage, recipient_channel_id);
        if channel_id.is_err() {
            return Err(ContractError::InvalidChannelId {});
        }

        match recipient.as_ref() {
            Some(recipient) => validate_hex(recipient, "recipient", None),
            None => Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                "missing recipient",
            ))),
        }?;
    }

    if let Some(recipient_ibc_channel_id) = recipient_ibc_channel_id {
        let ibc_channel_result: Result<String, cosmwasm_std::StdError> =
            crate::state::IBC_CHANNELS.load(storage, recipient_ibc_channel_id);
        if ibc_channel_result.is_err() {
            return Err(ContractError::InvalidIBCChannelId {});
        }

        let prefix = ibc_channel_result.unwrap();

        match recipient.as_ref() {
            Some(recipient) => {
                if recipient.starts_with(&prefix) {
                    Ok(())
                } else {
                    Err(ContractError::InvalidAddress {
                        kind: "recipient".into(),
                        address: recipient.to_string(),
                        reason: format!("missing {prefix} prefix"),
                    })
                }
            }
            None => Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                "missing recipient",
            ))),
        }?;
    }

    Ok(on_chain_recipient)
}

/// Errors:
/// - Returns contract error if zkgm channel id is 0
#[allow(clippy::type_complexity)]
pub fn split_and_validate_recipient(
    storage: &dyn Storage,
    api: &dyn Api,
    recipient: Recipient,
) -> Result<(Option<String>, Option<u32>, Option<String>), ContractError> {
    let (recipient_addr, recipient_channel_id, recipient_ibc_channel_id) = match recipient {
        Recipient::OnChain { address } => (Some(address.to_string()), None, None),
        Recipient::Zkgm {
            address,
            channel_id,
        } => {
            let recipient = Some(address);
            let recipient_channel_id = Some(channel_id);
            validate_recipient(storage, api, recipient.clone(), recipient_channel_id, None)?;
            (recipient, recipient_channel_id, None)
        }
        Recipient::Ibc {
            address,
            ibc_channel_id,
        } => {
            let recipient = Some(address);
            let recipient_ibc_channel_id = Some(ibc_channel_id);
            validate_recipient(
                storage,
                api,
                recipient.clone(),
                None,
                recipient_ibc_channel_id.clone(),
            )?;
            (recipient, None, recipient_ibc_channel_id)
        }
    };

    Ok((
        recipient_addr,
        recipient_channel_id,
        recipient_ibc_channel_id,
    ))
}

#[must_use]
pub fn is_on_chain_recipient(
    deps: &cosmwasm_std::Deps,
    recipient: &Option<String>,
    recipient_channel_id: Option<u32>,
    recipient_ibc_channel_id: &Option<String>,
) -> bool {
    let mut on_chain_recipient = false;
    if recipient.is_some() && recipient_channel_id.is_none() && recipient_ibc_channel_id.is_none() {
        let res = deps.api.addr_validate(recipient.clone().unwrap().as_str());
        if res.is_ok() {
            on_chain_recipient = true;
        }
    }

    on_chain_recipient
}

pub fn validate_required_coin(funds: &[Coin], min_bond: &Coin) -> Result<Coin, ContractError> {
    // coin must have be sent along with transaction and it should be in underlying coin denom
    let [coin] = funds else {
        return Err(ContractError::NoAsset {});
    };
    if coin.denom != min_bond.denom || coin.amount.is_zero() {
        return Err(ContractError::InvalidAsset {});
    }
    if coin.amount < min_bond.amount {
        return Err(ContractError::BondAmountTooLow {});
    }
    Ok(coin.clone())
}

pub fn validate_salt(salt: &str) -> Result<(), ContractError> {
    validate_hex(salt, "sal", Some(64))
}

pub fn validate_hex(
    value: &str,
    label_for_error: &str,
    length: Option<usize>,
) -> Result<(), ContractError> {
    let prefix = "0x";
    let hex = value
        .strip_prefix(prefix)
        .ok_or(ContractError::InvalidAddress {
            kind: label_for_error.into(),
            address: value.to_string(),
            reason: format!("missing {prefix} prefix"),
        })?;
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ContractError::InvalidAddress {
            kind: label_for_error.into(),
            address: value.to_string(),
            reason: "invalid hex chars".to_string(),
        });
    }
    if let Some(length) = length {
        let hex_len = hex.len();
        if hex_len != length {
            return Err(ContractError::InvalidAddress {
                kind: label_for_error.into(),
                address: value.to_string(),
                reason: format!("invalid length, expected {length}, got {hex_len}"),
            });
        }
    }
    Ok(())
}
