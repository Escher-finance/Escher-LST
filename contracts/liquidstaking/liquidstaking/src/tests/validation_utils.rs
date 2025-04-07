use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    Addr, Timestamp,
};

use crate::{
    state::{QuoteToken, Validator, LATEST_BOND_TIMESTAMPS},
    utils::validation::*,
    ContractError,
};

#[test]
fn test_validate_validators() {
    let deps = mock_dependencies();

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

    // Fails - zero weight
    validators[0].weight = 0;

    let err = validate_validators(&validators).unwrap_err();
    assert!(if let ContractError::InvalidValidators {} = err {
        true
    } else {
        false
    });

    validators[0].weight = 1;

    // Good
    validate_validators(&validators).unwrap();

    // Fails - repeated validator address
    let addr = validators[0].address.clone();
    validators.push(Validator {
        address: addr,
        weight: 10,
    });

    let err = validate_validators(&validators).unwrap_err();
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

#[test]
fn test_rate_limit_bond() {
    let mut deps = mock_dependencies();
    let mut env = mock_env();
    let lock_time_secs = 3600;
    let user = Addr::unchecked("user");

    let initial_block_time = Timestamp::from_seconds(10000);

    env.block.time = initial_block_time.clone();

    assert_eq!(
        LATEST_BOND_TIMESTAMPS
            .may_load(&deps.storage, user.clone())
            .unwrap(),
        None
    );

    // first time - should pass
    rate_limit_bond(deps.as_mut().storage, &env, lock_time_secs, user.clone()).unwrap();
    assert_eq!(
        LATEST_BOND_TIMESTAMPS
            .may_load(&deps.storage, user.clone())
            .unwrap(),
        Some(initial_block_time)
    );

    // not enough time has passed - should fail
    env.block.time = env.block.time.plus_seconds(lock_time_secs - 1);
    let err =
        rate_limit_bond(deps.as_mut().storage, &env, lock_time_secs, user.clone()).unwrap_err();
    assert!(matches!(
        err,
        ContractError::BondRateLimitExceeded { user: u } if user == u
    ));
    assert_eq!(
        LATEST_BOND_TIMESTAMPS
            .may_load(&deps.storage, user.clone())
            .unwrap(),
        Some(initial_block_time)
    );

    // enough time has passed - should pass
    env.block.time = env.block.time.plus_seconds(100);
    rate_limit_bond(deps.as_mut().storage, &env, lock_time_secs, user.clone()).unwrap();
    assert_eq!(
        LATEST_BOND_TIMESTAMPS
            .may_load(&deps.storage, user.clone())
            .unwrap(),
        Some(env.block.time)
    );
}
