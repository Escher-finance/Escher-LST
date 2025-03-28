use crate::state::Config;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct InstantiateMsg {
    pub lst_contract: Addr,
    pub fee_receiver: Addr,
    pub fee_rate: Decimal,
    pub coin_denom: String,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    SplitReward {},
    SetConfig {
        lst_contract_address: Option<Addr>,
        fee_receiver: Option<Addr>,
        fee_rate: Option<Decimal>,
        coin_denom: Option<String>,
    },
    TransferToOwner {},
}

#[cw_serde]
pub enum ExecuteLstMsg {
    Redelegate {},
}

#[cw_ownable_query]
#[cw_serde]
#[non_exhaustive]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
}

#[cw_serde]
pub enum LSTQueryMsg {
    RewardBalance {},
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct Balance {
    pub amount: Uint128,
}
