use std::collections::HashSet;

use crate::{
    state::{QuoteToken, Validator},
    ContractError,
};

pub fn validate_validators(validators: &Vec<Validator>) -> Result<(), ContractError> {
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

pub fn validate_quote_tokens(quote_tokens: &Vec<QuoteToken>) -> Result<(), ContractError> {
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

pub fn validate_recipient(
    deps: &cosmwasm_std::DepsMut,
    recipient: Option<String>,
    recipient_channel_id: Option<u32>,
    salt: Option<String>,
) -> Result<bool, ContractError> {
    let mut on_chain_recipient = false;
    // if recipient is provided but channel id is none, need to validate the address as it is the same chain address as contract
    if recipient.is_some() && recipient_channel_id.is_none() {
        deps.api
            .addr_validate(recipient.clone().unwrap().as_str())?;
        on_chain_recipient = true;
    }

    // if recipient_channel_id exists, must make sure the chain is supported
    if recipient_channel_id.is_some() {
        let channel_id = crate::state::CHAINS.load(deps.storage, recipient_channel_id.unwrap());
        if channel_id.is_err() {
            return Err(ContractError::InvalidChannelId {});
        }

        if salt.is_none() {
            return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                "missing salt",
            )));
        }
    }
    Ok(on_chain_recipient)
}

pub fn is_on_chain_recipient(
    deps: &cosmwasm_std::DepsMut,
    recipient: Option<String>,
    recipient_channel_id: Option<u32>,
) -> bool {
    let mut on_chain_recipient = false;
    if recipient.is_some() && recipient_channel_id.is_none() {
        let res = deps.api.addr_validate(recipient.clone().unwrap().as_str());
        if res.is_ok() {
            on_chain_recipient = true;
        }
    }

    on_chain_recipient
}
