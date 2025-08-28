use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

pub const MAX_TREASURY_FEE: Uint128 = Uint128::new(100_000);
/// The maximum allowed unbonding period is 42 days,
/// which is twice the typical staking period of a Cosmos SDK-based chain.
pub const MAX_UNBONDING_PERIOD: u64 = 3_628_800;

#[cw_serde]
pub struct BatchExpectedAmount {
    pub batch_id: u64,
    pub amount: Uint128,
}
