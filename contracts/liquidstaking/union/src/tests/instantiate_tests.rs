use std::marker::PhantomData;

use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_json_binary, ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest, SystemResult,
};
use milky_way::staking::BatchState;
use serde::Serialize;

use crate::{
    contract::instantiate,
    mock_deps_with_unbonding_time,
    state::{BATCHES, CONFIG},
    tests::test_helper::{mock_init_msg, OSMO3},
    types::MAX_TREASURY_FEE,
};

#[test]
fn invalid_native_token_denom_fails() {
    let mut deps = mock_dependencies();
    let info = mock_info(OSMO3, &[]);
    let mut msg = mock_init_msg();

    msg.protocol_fee_config.fee_rate = MAX_TREASURY_FEE;
    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg);
    assert!(res.is_err());
}

#[test]
fn init_properly() {
    let mut deps = mock_deps_with_unbonding_time!(100_000_000);
    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info(OSMO3, &[]).clone(),
        mock_init_msg().clone(),
    )
    .unwrap();

    assert_eq!(
        BATCHES.load(&deps.storage, 1).unwrap().state,
        BatchState::Pending {
            submit_time: mock_env().block.time.seconds() + mock_init_msg().batch_period
        },
    );
}
