use cosmwasm_std::{Uint128, testing::mock_dependencies};

use crate::{
    ContractError,
    query::*,
    state::{UnbondRecord, unbond_record},
};

#[test]
fn test_query_unbond_record() {
    let mut deps = mock_dependencies();
    let total = 30;
    let sender = "sender".to_string();
    let staker = "staker".to_string();
    for i in 0..total {
        let unbond_rec = UnbondRecord {
            id: i,
            height: 10000,
            sender: sender.clone(),
            staker: staker.clone(),
            channel_id: None,
            amount: Uint128::new(1000),
            released_height: 0,
            // These bellow are just to create some variation in the data
            released: i > (total / 2),
            batch_id: if i % 2 == 0 { 1 } else { 2 },
            recipient: None,
            recipient_channel_id: None,
        };
        unbond_record()
            .save(deps.as_mut().storage, i, &unbond_rec)
            .unwrap();
    }
    // Query by id
    let unbond_recs =
        query_unbond_record(&deps.storage, None, None, Some(20), None, None, None).unwrap();
    assert_eq!(unbond_recs.len(), 1);
    assert_eq!(unbond_recs[0].id, 20);
    // Query by batch_id
    let unbond_recs =
        query_unbond_record(&deps.storage, None, None, None, Some(2), None, None).unwrap();
    assert_eq!(unbond_recs.len(), total as usize / 2);
    assert!(unbond_recs.iter().all(|r| r.batch_id == 2));
    // Query by released
    let unbond_recs =
        query_unbond_record(&deps.storage, None, Some(false), None, None, None, None).unwrap();
    assert_eq!(unbond_recs.len(), total as usize / 2);
    assert!(unbond_recs.iter().all(|r| !r.released));
    // Query by staker
    let unbond_recs = query_unbond_record(
        &deps.storage,
        Some(staker.clone()),
        None,
        None,
        None,
        Some(10),
        Some(15),
    )
    .unwrap();
    assert_eq!(unbond_recs.len(), 6);
    assert!(
        unbond_recs
            .iter()
            .all(|r| r.staker == staker && r.id >= 10 && r.id <= 15)
    );
    // Query by staker_released
    let unbond_recs = query_unbond_record(
        &deps.storage,
        Some(staker.clone()),
        Some(true),
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(unbond_recs.len(), (total as usize / 2) - 1);
    assert!(unbond_recs.iter().all(|r| r.staker == staker && r.released));
}

#[test]
fn test_query_unbond_record_should_return_err_if_invalid_query() {
    use cosmwasm_std::testing::mock_dependencies;
    let deps = mock_dependencies();
    let err =
        query_unbond_record(&deps.storage, None, None, None, None, Some(0), Some(100)).unwrap_err();
    let has_right_error = matches!(err, ContractError::InvalidUnbondRecordQuery {});
    assert!(has_right_error);
}

#[test]
fn test_query_chains() {
    let mut deps = cosmwasm_std::testing::mock_dependencies();

    for i in 1..10 {
        let chain_id = format!("chain-{}", i);
        let data = crate::state::Chain {
            chain_id: chain_id.clone(),
            name: format!("chain{}", i),
            ucs03_channel_id: i,
            prefix: format!("b{}", i),
        };
        crate::state::CHAINS
            .save(&mut deps.storage, i, &data)
            .unwrap();
    }

    let chains: Vec<crate::state::Chain> = crate::state::CHAINS
        .range(&deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|result| result.ok().map(|(_, chain)| chain))
        .collect();

    let chain_1: &crate::state::Chain = chains.first().unwrap();
    assert_eq!(chain_1.ucs03_channel_id, 1);

    let chain_9: &crate::state::Chain = chains.get(8).unwrap();
    assert_eq!(chain_9.name, "chain9")
}
