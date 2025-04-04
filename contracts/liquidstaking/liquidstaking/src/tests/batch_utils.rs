use crate::utils::batch::*;
use cosmwasm_std::{testing::mock_dependencies, Uint128};

#[test]
fn test_batch_indexes() {
    let mut deps = mock_dependencies();
    let storage = deps.as_mut().storage;

    let batch_a = Batch {
        id: 0,
        total_liquid_stake: Uint128::new(1000),
        expected_native_unstaked: None,
        received_native_unstaked: None,
        unbond_records_count: 5,
        next_batch_action_time: Some(1000000),
        status: BatchStatus::Pending,
    };
    let batch_b = Batch {
        id: 1,
        total_liquid_stake: Uint128::new(2000),
        expected_native_unstaked: None,
        received_native_unstaked: None,
        unbond_records_count: 5,
        next_batch_action_time: None,
        status: BatchStatus::Received,
    };

    let batch_map = batches();
    batch_map.save(storage, batch_a.id, &batch_a).unwrap();
    batch_map.save(storage, batch_b.id, &batch_b).unwrap();

    assert_eq!(
        batch_map.load(storage, batch_b.id).unwrap().status,
        batch_b.status
    );
}

#[test]
fn test_batch_status() {
    assert_eq!(BatchStatus::Pending.to_string(), "pending");
    assert_eq!(BatchStatus::Submitted.to_string(), "submitted");
    assert_eq!(BatchStatus::Received.to_string(), "received");
    assert_eq!(BatchStatus::Released.to_string(), "released");
}

#[test]
fn test_batch() {
    let mut next_batch_action_time = 10;
    let mut batch = Batch::new(0, Uint128::new(1000), next_batch_action_time);
    assert_eq!(batch.status, BatchStatus::Pending);
    assert_eq!(batch.next_batch_action_time, Some(next_batch_action_time));

    next_batch_action_time += 10;
    batch.update_status(BatchStatus::Pending, Some(next_batch_action_time));
    assert_eq!(batch.status, BatchStatus::Pending);
    assert_eq!(batch.next_batch_action_time, Some(next_batch_action_time));

    next_batch_action_time += 10;
    batch.update_status(BatchStatus::Submitted, Some(next_batch_action_time));
    assert_eq!(batch.status, BatchStatus::Submitted);
    assert_eq!(batch.next_batch_action_time, Some(next_batch_action_time));

    batch.update_status(BatchStatus::Received, Some(next_batch_action_time));
    assert_eq!(batch.status, BatchStatus::Received);
    assert_eq!(batch.next_batch_action_time, None);

    batch.update_status(BatchStatus::Released, Some(next_batch_action_time));
    assert_eq!(batch.status, BatchStatus::Released);
    assert_eq!(batch.next_batch_action_time, None);
}
