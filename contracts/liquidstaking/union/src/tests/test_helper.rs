use cosmwasm_std::{
    coins,
    testing::{message_info, mock_env, MockApi, MockStorage},
    Addr, OwnedDeps, Uint128,
};
use prost::Message;

use crate::{
    contract::instantiate,
    msg::InstantiateMsg,
    state::{ProtocolFeeConfig, CONFIG},
};

pub const NATIVE_TOKEN: &str = "au";
pub const ADMIN: &str = "union1fktal7292h36h7glff5edq59vpdfn7504duw5m";
pub const UNION1: &str = "union1jk9psyhvgkrt2cumz8eytll2244m2nnz4yt2g2";
pub const UNION2: &str = "union1a8k05kaazq576sd0n07ewhsplwtpecxjx8ygx9";
pub const UNION3: &str = "union1f00pmpma555uunmcsy95kartescpeyuqrd2y7z";
pub const UNION_STAKER: &str = "union1qp4uzhet2sd9mrs46kemse5dt9ncz4k3hjst5m";
pub const UNION_MONITOR_1: &str = "union1m27enjqfejnnxru5snmcufway88py4fyqcr43u";
pub const UNION_MONITOR_2: &str = "union1n36766lfkjcdwawu23d2tszwm26acesthzx0y7";
pub const LIQUID_STAKE_TOKEN_ADDRESS: &str = "union1wr7yz3mcyd6qe3c9z5mruxv4fq99z9sg0ks6hs";
pub const FUNDED_DISPATCH_ADDRESS: &str =
    "union1hnuj8f6d3wy3fcprt55vddv7v2650t6uudnvd2hukqrteeam8wjqvcmecf";
pub const ZKGM_ADDRESS: &str = "union1xwfgw7n6vwgkyv8syjskzak7lh8kmrcthmv2jsmywhyunekmg3zqul8vsh";

pub fn mock_init_msg() -> InstantiateMsg {
    InstantiateMsg {
        staker_address: Addr::unchecked(UNION_STAKER),
        minimum_liquid_stake_amount: Uint128::from(100u128),
        liquid_stake_token_address: LIQUID_STAKE_TOKEN_ADDRESS.to_string(),
        monitors: vec![
            Addr::unchecked(UNION_MONITOR_1),
            Addr::unchecked(UNION_MONITOR_2),
        ],
        batch_period: 86400,
        protocol_fee_config: ProtocolFeeConfig {
            fee_rate: Uint128::from(10_000u128),
            fee_recipient: Addr::unchecked(UNION_STAKER),
        },
        admin: Addr::unchecked(ADMIN),
        native_token_denom: "au".to_owned(),
        reward_collector_address: Addr::unchecked(UNION_STAKER),
        ucs03_zkgm_address: Addr::unchecked(ZKGM_ADDRESS),
        funded_dispatch_address: Addr::unchecked(FUNDED_DISPATCH_ADDRESS),
    }
}

pub fn init() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_deps_with_unbonding_time(100_000);
    let msg = mock_init_msg();
    let info = message_info(&Addr::unchecked(ADMIN), &coins(1000, "uosmo"));

    let res = instantiate(deps.as_mut(), mock_env(), info, msg);
    if res.is_err() {
        panic!("error: {:?}", res);
    }
    assert!(res.is_ok());

    let mut config = CONFIG.load(&deps.storage).unwrap();
    config.stopped = false;
    CONFIG.save(&mut deps.storage, &config).unwrap();

    deps
}

#[derive(Clone, PartialEq, Message)]
pub struct Duration {
    #[prost(int64, tag = "1")]
    pub seconds: i64,
}

#[derive(Clone, PartialEq, Message)]
pub struct Params {
    #[prost(message, optional, tag = "1")]
    pub unbonding_time: Option<Duration>,
}

#[derive(Clone, PartialEq, Message)]
pub struct QueryParamsResponse {
    #[prost(message, optional, tag = "1")]
    pub params: Option<Params>,
}

#[derive(Clone, Default)]
pub struct MockQuerier {
    unbonding_time: i64,
}

impl cosmwasm_std::Querier for MockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> cosmwasm_std::QuerierResult {
        // Deserialize the request
        let query: cosmwasm_std::QueryRequest = cosmwasm_std::from_json(bin_request).unwrap();

        match query {
            // this query is meant to be used for getting the unbonding time
            cosmwasm_std::QueryRequest::Grpc(_) => {
                cosmwasm_std::SystemResult::Ok(cosmwasm_std::ContractResult::Ok(
                    prost::Message::encode_to_vec(
                        &crate::tests::test_helper::QueryParamsResponse {
                            params: Some(crate::tests::test_helper::Params {
                                unbonding_time: Some(crate::tests::test_helper::Duration {
                                    seconds: self.unbonding_time,
                                }),
                            }),
                        },
                    )
                    .into(),
                ))
            }
            _ => panic!("unexpected query"),
        }
    }
}

pub fn mock_deps_with_unbonding_time(
    unbonding_time: i64,
) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    cosmwasm_std::OwnedDeps {
        storage: cosmwasm_std::testing::MockStorage::default(),
        api: cosmwasm_std::testing::MockApi::default().with_prefix("union"),
        querier: MockQuerier { unbonding_time },
        custom_query_type: std::marker::PhantomData,
    }
}
