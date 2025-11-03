use cosmwasm_std::{Coin, Uint128, testing::mock_dependencies};

use crate::{
    ContractError,
    msg::Recipient,
    state::{BOND_ZKGM_CHAINS, IBC_CHANNELS, QuoteToken, Validator, ZkgmChain},
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
    let deps = mock_dependencies();
    let recipient = deps.api.addr_make("user");

    // invalid recipient
    assert!(
        validation::validate_recipient(
            &deps.storage,
            &deps.api,
            Some(&"invalid".to_string()),
            None,
            None,
            &crate::msg::RecipientAction::Bond
        )
        .is_err()
    );

    // missing recipient
    assert!(
        validation::validate_recipient(
            &deps.storage,
            &deps.api,
            None,
            None,
            None,
            &crate::msg::RecipientAction::Bond
        )
        .is_err()
    );

    let on_chain_recipient = validation::validate_recipient(
        &deps.storage,
        &deps.api,
        Some(&recipient.to_string()),
        None,
        None,
        &crate::msg::RecipientAction::Unbond,
    )
    .unwrap();
    assert_eq!(on_chain_recipient, true);
}

#[test]
fn test_validate_recipient_channel_id() {
    let mut deps = mock_dependencies();
    let channel_id = 1;

    let chain = ZkgmChain {
        prefix: "cosmwasm".to_string(),
        name: "chain".to_string(),
        chain_id: "chain-1".to_string(),
        ucs03_channel_id: channel_id,
    };
    BOND_ZKGM_CHAINS
        .save(&mut deps.storage, channel_id, &chain)
        .unwrap();

    let recipient = "0xeeEEeeE98622c19Ea39Ea8827ae22Bbfc732671c";

    // unknown channel_id
    assert!(
        validation::validate_recipient(
            &deps.storage,
            &deps.api,
            Some(&recipient.to_string()),
            Some(5),
            None,
            &crate::msg::RecipientAction::Bond
        )
        .is_err()
    );

    // invalid channel_id
    assert!(
        validation::validate_recipient(
            &deps.storage,
            &deps.api,
            Some(&recipient.to_string()),
            Some(0),
            None,
            &crate::msg::RecipientAction::Bond
        )
        .is_err()
    );

    // missing recipient
    assert!(
        validation::validate_recipient(
            &deps.storage,
            &deps.api,
            None,
            Some(channel_id),
            None,
            &crate::msg::RecipientAction::Bond
        )
        .is_err()
    );

    // invalid recipient
    assert!(
        validation::validate_recipient(
            &deps.storage,
            &deps.api,
            Some(&"invalid".to_string()),
            Some(channel_id),
            None,
            &crate::msg::RecipientAction::Bond
        )
        .is_err()
    );

    let on_chain_recipient = validation::validate_recipient(
        &deps.storage,
        &deps.api,
        Some(&recipient.to_string()),
        Some(channel_id),
        None,
        &crate::msg::RecipientAction::Bond,
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
            &"osmo".to_string(),
        )
        .unwrap();

    let recipient = "osmo185fflsvwrz0cx46w6qada7mdy92m6kx4qm4l9k";

    // unknown channel_id
    assert!(
        validation::validate_recipient(
            &deps.storage,
            &deps.api,
            Some(&recipient.to_string()),
            None,
            Some("channel-2".to_string()),
            &crate::msg::RecipientAction::Unbond
        )
        .is_err()
    );

    // missing recipient
    assert!(
        validation::validate_recipient(
            &deps.storage,
            &deps.api,
            None,
            None,
            Some(ibc_channel_id.clone()),
            &crate::msg::RecipientAction::Unbond
        )
        .is_err()
    );

    // invalid recipient
    assert!(
        validation::validate_recipient(
            &deps.storage,
            &deps.api,
            Some(&"invalid".to_string()),
            None,
            Some(ibc_channel_id.clone()),
            &crate::msg::RecipientAction::Unbond
        )
        .is_err()
    );

    let on_chain_recipient = validation::validate_recipient(
        &deps.storage,
        &deps.api,
        Some(&recipient.to_string()),
        None,
        Some(ibc_channel_id.clone()),
        &crate::msg::RecipientAction::Unbond,
    )
    .unwrap();
    assert_eq!(on_chain_recipient, false);
}

#[test]
fn test_split_and_validate_recipient_on_chain() {
    let deps = mock_dependencies();
    let recipient_addr = deps.api.addr_make("user");
    let recipient = Recipient::OnChain {
        address: recipient_addr.clone(),
    };
    let result = validation::split_and_validate_recipient(
        &deps.storage,
        &deps.api,
        recipient,
        &crate::msg::RecipientAction::Bond,
    )
    .unwrap();
    assert_eq!(result.0, Some(recipient_addr.to_string()));
    assert_eq!(result.1, None);
    assert_eq!(result.2, None);
}

#[test]
fn test_split_and_validate_recipient_zkgm() {
    let mut deps = mock_dependencies();
    let channel_id = 1;
    let recipient_addr = "0xeeEEeeE98622c19Ea39Ea8827ae22Bbfc732671c".to_string();
    let recipient = Recipient::Zkgm {
        address: recipient_addr.clone(),
        channel_id,
    };

    let chain = ZkgmChain {
        prefix: "cosmwasm".to_string(),
        name: "chain".to_string(),
        chain_id: "chain-1".to_string(),
        ucs03_channel_id: channel_id,
    };
    BOND_ZKGM_CHAINS
        .save(&mut deps.storage, channel_id, &chain)
        .unwrap();

    let result = validation::split_and_validate_recipient(
        &deps.storage,
        &deps.api,
        recipient,
        &crate::msg::RecipientAction::Bond,
    )
    .unwrap();
    assert_eq!(result.0, Some(recipient_addr.to_string()));
    assert_eq!(result.1, Some(channel_id));
    assert_eq!(result.2, None);
}

#[test]
fn test_split_and_validate_recipient_ibc() {
    let mut deps = mock_dependencies();
    let ibc_channel_id = "channel-1".to_string();
    let recipient_addr = "osmo185fflsvwrz0cx46w6qada7mdy92m6kx4qm4l9k".to_string();
    let recipient = Recipient::Ibc {
        address: recipient_addr.clone(),
        ibc_channel_id: ibc_channel_id.clone(),
    };

    IBC_CHANNELS
        .save(
            &mut deps.storage,
            ibc_channel_id.clone(),
            &"osmo".to_string(),
        )
        .unwrap();

    let result = validation::split_and_validate_recipient(
        &deps.storage,
        &deps.api,
        recipient.clone(),
        &crate::msg::RecipientAction::Unbond,
    )
    .unwrap();
    assert_eq!(result.0, Some(recipient_addr.to_string()));
    assert_eq!(result.1, None);
    assert_eq!(result.2, Some(ibc_channel_id));
}

#[test]
fn test_validate_hex() {
    let value = "0xeeEEeeE98622c19Ea339Ea8827ae22Bbfc732671ce9Ea8827ae22Bbfc732671c".to_string();

    // bad length
    assert!(validation::validate_hex(&value, "hex", Some(63)).is_err());

    // missing prefix
    assert!(validation::validate_hex(&value.strip_prefix("0x").unwrap(), "hex", None).is_err());

    // invalid hex chars
    assert!(validation::validate_hex("0xescher", "hex", None).is_err());

    validation::validate_hex(&value, "hex", Some(64)).unwrap();
    validation::validate_hex(&value, "hex", None).unwrap();
}

#[test]
fn test_validate_salt() {
    let value = "0xe5cf1e5cf1e5cf1e5cf1e5cf1e5cf1e5cf1e5cf1e5cf1e5cf1e5cf1e5cf1e5cf".to_string();

    // bad length
    assert!(validation::validate_salt(&value[..value.len() - 2]).is_err());

    validation::validate_salt(&value).unwrap();
}

#[test]
fn test_validate_required_salt() {
    let value = "0xe5cf1e5cf1e5cf1e5cf1e5cf1e5cf1e5cf1e5cf1e5cf1e5cf1e5cf1e5cf1e5cf".to_string();

    // missing salt
    assert!(validation::validate_required_salt(&None).is_err());

    assert_eq!(
        validation::validate_required_salt(&Some(value.clone())).unwrap(),
        value
    );
}

#[test]
fn test_validate_required_coin() {
    let coin = Coin::new(100_u128, "a".to_string());
    let coin_other = Coin::new(100_u128, "b".to_string());

    // invalid coin
    assert!(validation::validate_required_coin(&[coin_other.clone()], &coin).is_err());

    // insufficient amount
    assert!(
        validation::validate_required_coin(
            &[coin.clone()],
            &Coin::new(coin.amount + Uint128::one(), coin.denom.clone()),
        )
        .is_err()
    );

    // multiple coins
    assert!(
        validation::validate_required_coin(&[coin.clone(), coin_other.clone()], &coin).is_err()
    );

    validation::validate_required_coin(&[coin.clone()], &coin).unwrap();
    validation::validate_required_coin(
        &[coin.clone()],
        &Coin::new(coin.amount - Uint128::one(), coin.denom.clone()),
    )
    .unwrap();
}

#[test]
fn test_is_valid_cosmos_address() {
    // Valid addresses with correct prefix and length
    assert!(validation::is_valid_cosmos_address(
        "bbn1fju94gpxlcg5tqxp2sf0c9ns6gkxqcsd5cezm2",
        "bbn"
    ));

    // Valid contract address with correct prefix and length
    assert!(validation::is_valid_cosmos_address(
        "bbn1l2um8k69h9sd2amtypp0c44d5zxgylp8z84lmfujjvejw5k6frfswwr3wp",
        "bbn"
    ));

    // Valid addresses with correct prefix and length
    assert!(validation::is_valid_cosmos_address(
        "cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuux",
        "cosmos"
    ));

    // Valid addresses with correct prefix and length
    assert!(validation::is_valid_cosmos_address(
        "osmo1qw6npqrhgt0k4wvjecyggsyy0u492sg26wwtgttrmwc2xxelghgqkykpf9",
        "osmo"
    ));

    // Invalid: empty string
    assert!(!validation::is_valid_cosmos_address("", "cosmos"));

    // Invalid: wrong prefix
    assert!(!validation::is_valid_cosmos_address(
        "cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuux",
        "osmo"
    ));
    assert!(!validation::is_valid_cosmos_address(
        "osmo1syavy2npfyt9tcncdtsdzf7kny9lh777vsd4zx",
        "cosmos"
    ));

    // Invalid: incorrect length (too short)
    assert!(!validation::is_valid_cosmos_address(
        "cosmos1syavy2npfyt9tcncdtsdzf",
        "cosmos"
    ));

    // Invalid: incorrect length
    assert!(!validation::is_valid_cosmos_address(
        "cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuuxextralong",
        "cosmos"
    ));

    // Invalid: not a bech32 encoded address
    assert!(!validation::is_valid_cosmos_address(
        "not_a_valid_address",
        "cosmos"
    ));

    // Invalid: hex address instead of bech32
    assert!(!validation::is_valid_cosmos_address(
        "0xeeEEeeE98622c19Ea39Ea8827ae22Bbfc732671c",
        "cosmos"
    ));

    // Invalid: contains invalid bech32 characters
    assert!(!validation::is_valid_cosmos_address(
        "cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuuB",
        "cosmos"
    ));

    // Edge case: prefix only
    assert!(!validation::is_valid_cosmos_address("cosmos", "cosmos"));

    // Edge case: prefix with separator but no data
    assert!(!validation::is_valid_cosmos_address("cosmos1", "cosmos"));
}
