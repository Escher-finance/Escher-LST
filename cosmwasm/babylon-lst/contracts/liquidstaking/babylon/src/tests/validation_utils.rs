use cosmwasm_std::testing::mock_dependencies;

use crate::{
    ContractError,
    state::{QuoteToken, Validator},
    utils::validation::*,
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
    assert!(matches!(err, ContractError::InvalidValidators {}));

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
    assert!(matches!(err, ContractError::InvalidValidators {}));
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
    assert!(matches!(err, ContractError::InvalidQuoteTokens {}));
}

#[test]
fn test_is_on_chain_recipient() {
    let deps = mock_dependencies();

    let recipient_addr = deps.api.addr_make("isak");
    let recipient = Some(recipient_addr.to_string());
    let recipient_channel_id = None;
    let is_same_chain_recipient = crate::utils::validation::is_on_chain_recipient(
        &deps.as_ref(),
        &recipient,
        recipient_channel_id,
        &None,
    );

    println!("recipient_addr: {recipient_addr}");
    println!("is_same_chain_recipient: {is_same_chain_recipient}");
    assert!(is_same_chain_recipient);

    let recipient = Some("0xbb74285235846c9d98280ac92ab8007382e51234".to_string());
    let is_same_chain_recipient = crate::utils::validation::is_on_chain_recipient(
        &deps.as_ref(),
        &recipient,
        recipient_channel_id,
        &None,
    );
    assert!(!is_same_chain_recipient);

    let recipient = Some("uniondefghabcuxz7l0vcusq5jc9zvzpm8ec2au39x123".to_string());
    let is_same_chain_recipient = crate::utils::validation::is_on_chain_recipient(
        &deps.as_ref(),
        &recipient,
        recipient_channel_id,
        &None,
    );
    assert!(!is_same_chain_recipient);
}
