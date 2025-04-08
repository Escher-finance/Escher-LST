use std::collections::HashSet;

use cosmwasm_std::{Env, Storage};

use crate::{
    state::{Action, QuoteToken, Validator, ACTION_TIMESTAMPS},
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

    if validators
        .iter()
        .find(|validator| validator.weight == 0)
        .is_some()
    {
        return Err(ContractError::InvalidValidators {});
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

pub fn rate_limit(
    storage: &mut dyn Storage,
    env: &Env,
    lock_time_secs: u64,
    user: String,
    action: Action,
) -> Result<(), ContractError> {
    let action_key = format!("{action}-{user}");
    let latest_time = ACTION_TIMESTAMPS
        .may_load(storage, action_key.clone())?
        .unwrap_or_default();

    let current_time = env.block.time;

    if current_time < latest_time.plus_seconds(lock_time_secs) {
        return Err(ContractError::RateLimitExceeded { action, user });
    }

    ACTION_TIMESTAMPS.save(storage, action_key, &current_time)?;

    Ok(())
}
