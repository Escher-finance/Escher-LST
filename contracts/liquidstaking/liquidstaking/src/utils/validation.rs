use std::collections::HashSet;

use cosmwasm_std::{Addr, Env, Storage};

use crate::{
    state::{QuoteToken, Validator, LATEST_BOND_TIMESTAMPS},
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

pub fn rate_limit_bond(
    storage: &mut dyn Storage,
    env: &Env,
    lock_time_secs: u64,
    user: Addr,
) -> Result<(), ContractError> {
    let latest_bond_time = LATEST_BOND_TIMESTAMPS
        .may_load(storage, user.clone())?
        .unwrap_or_default();

    let current_time = env.block.time;

    if current_time < latest_bond_time.plus_seconds(lock_time_secs) {
        return Err(ContractError::BondRateLimitExceeded { user });
    }

    LATEST_BOND_TIMESTAMPS.save(storage, user.clone(), &current_time)?;

    Ok(())
}
