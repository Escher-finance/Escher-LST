use crate::{query::*, ContractError};

#[test]
fn test_query_unbond_record_should_return_err_if_invalid_query() {
    use cosmwasm_std::testing::mock_dependencies;
    let deps = mock_dependencies();
    let err = query_unbond_record(&deps.storage, None, None, None, Some(0), Some(100)).unwrap_err();
    let has_right_error = if let ContractError::InvalidUnbondRecordQuery {} = err {
        true
    } else {
        false
    };
    assert!(has_right_error);
}
