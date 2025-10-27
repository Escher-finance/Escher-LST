use cosmwasm_std::testing::mock_dependencies;

use crate::{
    ContractError,
    state::{CHAINS, Chain, IBC_CHANNELS, QuoteToken, Validator},
    utils::validation::{self, *},
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

#[test]
fn test_validate_recipient_on_chain_recipient() {
    let mut deps = mock_dependencies();
    let recipient = deps.api.addr_make("user");

    // invalid recipient
    assert!(
        validation::validate_recipient(
            &deps.as_mut(),
            Some("invalid".to_string()),
            None,
            None,
            &None,
        )
        .is_err()
    );

    // missing recipient
    assert!(validation::validate_recipient(&deps.as_mut(), None, None, None, &None).is_err());

    let on_chain_recipient = validation::validate_recipient(
        &deps.as_mut(),
        Some(recipient.to_string()),
        None,
        None,
        &None,
    )
    .unwrap();
    assert_eq!(on_chain_recipient, true);
}

#[test]
fn test_validate_recipient_channel_id() {
    let mut deps = mock_dependencies();
    let channel_id = 1;

    let chain = Chain {
        prefix: "cosmwasm".to_string(),
        name: "chain".to_string(),
        chain_id: "chain-1".to_string(),
        ucs03_channel_id: channel_id,
    };
    CHAINS.save(&mut deps.storage, channel_id, &chain).unwrap();

    let recipient = "0xeeEEeeE98622c19Ea39Ea8827ae22Bbfc732671c";
    let salt = "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    // unknown channel_id
    assert!(
        validation::validate_recipient(
            &deps.as_mut(),
            Some(recipient.to_string()),
            Some(5),
            None,
            &Some(salt.to_string()),
        )
        .is_err()
    );

    // missing recipient
    assert!(
        validation::validate_recipient(
            &deps.as_mut(),
            None,
            Some(channel_id),
            None,
            &Some(salt.to_string()),
        )
        .is_err()
    );

    // invalid recipient
    assert!(
        validation::validate_recipient(
            &deps.as_mut(),
            Some("invalid".to_string()),
            Some(channel_id),
            None,
            &Some(salt.to_string()),
        )
        .is_err()
    );

    // missing salt
    assert!(
        validation::validate_recipient(
            &deps.as_mut(),
            Some(recipient.to_string()),
            Some(channel_id),
            None,
            &None,
        )
        .is_err()
    );

    // invalid salt
    assert!(
        validation::validate_recipient(
            &deps.as_mut(),
            Some(recipient.to_string()),
            Some(channel_id),
            None,
            &Some("salt".to_string()),
        )
        .is_err()
    );

    let on_chain_recipient = validation::validate_recipient(
        &deps.as_mut(),
        Some(recipient.to_string()),
        Some(channel_id),
        None,
        &Some(salt.to_string()),
    )
    .unwrap();
    assert_eq!(on_chain_recipient, false);
}

#[test]
fn test_validate_recipient_ibc_channel_id() {
    let mut deps = mock_dependencies();
    let ibc_channel_id = "channel-1".to_string();

    IBC_CHANNELS
        .save(
            &mut deps.storage,
            ibc_channel_id.clone(),
            &"cosmwasm".to_string(),
        )
        .unwrap();

    let recipient = "cosmwasmeeeeeeeeeeee";
    let salt = "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    // unknown channel_id
    assert!(
        validation::validate_recipient(
            &deps.as_mut(),
            Some(recipient.to_string()),
            None,
            Some("channel-2".to_string()),
            &Some(salt.to_string()),
        )
        .is_err()
    );

    // missing recipient
    assert!(
        validation::validate_recipient(
            &deps.as_mut(),
            None,
            None,
            Some(ibc_channel_id.clone()),
            &Some(salt.to_string()),
        )
        .is_err()
    );

    // invalid recipient
    assert!(
        validation::validate_recipient(
            &deps.as_mut(),
            Some("invalid".to_string()),
            None,
            Some(ibc_channel_id.clone()),
            &Some(salt.to_string()),
        )
        .is_err()
    );

    let on_chain_recipient = validation::validate_recipient(
        &deps.as_mut(),
        Some(recipient.to_string()),
        None,
        Some(ibc_channel_id.clone()),
        &None,
    )
    .unwrap();
    assert_eq!(on_chain_recipient, false);
}
