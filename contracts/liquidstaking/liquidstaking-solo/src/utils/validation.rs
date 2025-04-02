use std::collections::HashSet;

use cosmwasm_std::DepsMut;

use crate::{
    state::{QuoteToken, Validator},
    ContractError,
};

pub fn validate_validators(
    _deps: &DepsMut,
    validators: &Vec<Validator>,
) -> Result<(), ContractError> {
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

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_dependencies;

    use super::*;

    #[test]
    fn test_validate_validators() {
        let mut deps = mock_dependencies();

        let mut validators = Vec::from([
            Validator {
                address: "a".to_string(),
                weight: 1,
            },
            Validator {
                address: deps.api.addr_make("b").to_string(),
                weight: 10,
            },
            Validator {
                address: deps.api.addr_make("c").to_string(),
                weight: 10,
            },
        ]);

        // Fails - bad addr
        let err = validate_validators(&deps.as_mut(), &validators).unwrap_err();
        assert!(if let ContractError::Std(_) = err {
            true
        } else {
            false
        });

        validators[0].address = deps.api.addr_make("a").to_string();
        validators[0].weight = 0;

        // Fails - zero weight
        let err = validate_validators(&deps.as_mut(), &validators).unwrap_err();
        assert!(if let ContractError::InvalidValidators {} = err {
            true
        } else {
            false
        });

        validators[0].weight = 1;

        // Good
        validate_validators(&deps.as_mut(), &validators).unwrap();

        // Fails - repeated validator address
        let addr = validators[0].address.clone();
        validators.push(Validator {
            address: addr,
            weight: 10,
        });

        let err = validate_validators(&deps.as_mut(), &validators).unwrap_err();
        assert!(if let ContractError::InvalidValidators {} = err {
            true
        } else {
            false
        });
    }

    #[test]
    fn test_validate_quote_tokens() {
        let mut quote_tokens = Vec::from([
            QuoteToken {
                channel_id: 1,
                quote_token: "a".to_string(),
                lst_quote_token: "b".to_string(),
            },
            QuoteToken {
                channel_id: 2,
                quote_token: "c".to_string(),
                lst_quote_token: "d".to_string(),
            },
        ]);

        // Good
        validate_quote_tokens(&quote_tokens).unwrap();

        let channel_id = quote_tokens[0].channel_id;
        quote_tokens.push(QuoteToken {
            channel_id,
            quote_token: "e".to_string(),
            lst_quote_token: "f".to_string(),
        });

        // Fails - repeated quote token channel_id
        let err = validate_quote_tokens(&quote_tokens).unwrap_err();
        assert!(if let ContractError::InvalidQuoteTokens {} = err {
            true
        } else {
            false
        });
    }
}
