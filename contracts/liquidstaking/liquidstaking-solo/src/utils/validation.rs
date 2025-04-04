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
