use std::collections::HashSet;

use cosmwasm_std::{Addr, Coin, QuerierWrapper, Uint128};

use crate::{
    ContractError,
    state::{Parameters, QuoteToken, Validator},
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
/// - Returns address/channel validation errors or missing salt errors.
pub fn validate_recipient(
    deps: &cosmwasm_std::DepsMut,
    recipient: Option<String>,
    recipient_channel_id: Option<u32>,
    recipient_ibc_channel_id: Option<String>,
    salt: Option<String>,
) -> Result<bool, ContractError> {
    let mut on_chain_recipient = false;
    // if recipient is provided but channel id is none, need to validate the address as it is the same chain address as contract
    if recipient.is_some() && recipient_channel_id.is_none() && recipient_ibc_channel_id.is_none() {
        deps.api
            .addr_validate(recipient.clone().unwrap().as_str())?;
        on_chain_recipient = true;
    }

    // if recipient_channel_id exists, must make sure the chain is supported and recipient address is in hex
    if let Some(recipient_channel_id) = recipient_channel_id {
        let channel_id = crate::state::CHAINS.load(deps.storage, recipient_channel_id);
        if channel_id.is_err() {
            return Err(ContractError::InvalidChannelId {});
        }

        if !recipient.clone().unwrap().starts_with("0x") {
            return Err(ContractError::InvalidAddress {
                kind: "recipient".into(),
                address: recipient.unwrap(),
                reason: "address must be in hex and starts with 0x".to_string(),
            });
        }

        if salt.is_none() {
            return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                "missing salt",
            )));
        }
    }

    if let Some(recipient_ibc_channel_id) = recipient_ibc_channel_id {
        let ibc_channel_result: Result<String, cosmwasm_std::StdError> =
            crate::state::IBC_CHANNELS.load(deps.storage, recipient_ibc_channel_id);
        if ibc_channel_result.is_err() {
            return Err(ContractError::InvalidIBCChannelId {});
        }

        let prefix = ibc_channel_result.unwrap();

        if !recipient.clone().unwrap().starts_with(&prefix) {
            return Err(ContractError::InvalidAddress {
                kind: "recipient".into(),
                address: recipient.unwrap(),
                reason: format!("address prefix must starts with {prefix}"),
            });
        }
    }
    Ok(on_chain_recipient)
}

#[must_use]
pub fn is_on_chain_recipient(
    deps: &cosmwasm_std::Deps,
    recipient: Option<String>,
    recipient_channel_id: Option<u32>,
    recipient_ibc_channel_id: Option<String>,
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

/// Errors:
/// - Returns `InvalidAddress` when sender is not the expected cw-account contract.
pub fn validate_remote_sender(
    querier: QuerierWrapper,
    sender: &Addr,
    params: Parameters,
) -> Result<(), ContractError> {
    // assume sender is cw-account contract address if contract creator is the ucs03 contract
    match querier.query_wasm_contract_info(sender) {
        Ok(contract_info) => {
            if contract_info.creator.to_string() != params.ucs03_relay_contract {
                return Err(ContractError::InvalidAddress {
                    kind: "remote_bond".to_string(),
                    address: sender.to_string(),
                    reason: "not cw-account contract".to_string(),
                });
            }
        }
        Err(_) => {
            return Err(ContractError::InvalidAddress {
                kind: "remote_bond".to_string(),
                address: sender.to_string(),
                reason: "not a contract".to_string(),
            });
        }
    }
    Ok(())
}

pub fn validate_required_coin(funds: &[Coin], min_bond: &Coin) -> Result<Coin, ContractError> {
    // coin must have be sent along with transaction and it should be in underlying coin denom
    if funds.len() > 1usize {
        return Err(ContractError::InvalidAsset {});
    }
    let coin = funds
        .iter()
        .find(|x| x.denom == min_bond.denom && x.amount > Uint128::zero())
        .cloned()
        .ok_or_else(|| ContractError::NoAsset {})?;
    if coin.amount < min_bond.amount {
        return Err(ContractError::BondAmountTooLow {});
    }
    Ok(coin)
}
