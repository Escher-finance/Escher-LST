use cosmwasm_std::testing::{mock_dependencies, mock_info};
use milky_way::staking::BatchState;

use crate::{
    contract::instantiate,
    state::BATCHES,
    tests::test_helper::{mock_init_msg, OSMO3},
    types::MAX_TREASURY_FEE,
};

#[test]
fn invalid_native_token_denom_fails() {
    let mut deps = mock_dependencies();
    let info = mock_info(OSMO3, &[]);
    let mut msg = mock_init_msg();

    msg.protocol_fee_config.fee_rate = MAX_TREASURY_FEE;
    let res = instantiate(
        deps.as_mut(),
        cosmwasm_std::testing::mock_env(),
        info.clone(),
        msg,
    );
    assert!(res.is_err());
}

#[test]
fn init_properly() {
    let mut deps = mock_dependencies();
    instantiate(
        deps.as_mut(),
        cosmwasm_std::testing::mock_env(),
        mock_info(OSMO3, &[]).clone(),
        mock_init_msg().clone(),
    )
    .unwrap();

    assert_eq!(
        BATCHES.load(&deps.storage, 1).unwrap().state,
        BatchState::Pending { submit_time: 1 },
    );
}
