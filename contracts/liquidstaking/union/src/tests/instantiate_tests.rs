use cosmwasm_std::{
    testing::{message_info, mock_dependencies, mock_env},
    Addr,
};

use crate::{
    contract::instantiate,
    state::BATCHES,
    tests::test_helper::{mock_deps_with_unbonding_time, mock_init_msg, UNION1},
    types::{BatchState, MAX_TREASURY_FEE},
};

#[test]
fn invalid_native_token_denom_fails() {
    let mut deps = mock_dependencies();
    let info = message_info(&Addr::unchecked(UNION1), &[]);
    let mut msg = mock_init_msg();

    msg.protocol_fee_config.fee_rate = MAX_TREASURY_FEE;
    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg);
    assert!(res.is_err());
}

#[test]
fn init_properly() {
    let mut deps = mock_deps_with_unbonding_time(100_000_000);
    instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&Addr::unchecked(UNION1), &[]).clone(),
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
