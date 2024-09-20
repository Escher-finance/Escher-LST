use crate::state::{Config, Parameters, State, ValidatorsRegistry};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    pub underlying_coin_denom: String,
    pub validators: Vec<Addr>,
}

#[cw_serde]
pub enum ExecuteMsg {
    ////////////////////
    /// Owner's operations
    ////////////////////
    UpdateConfig {
        owner: Option<String>,
        validators: Option<String>,
    },

    ////////////////////
    /// User's operations
    ////////////////////

    /// Receives `amount` in underlying coin denom from sender.
    /// Delegate `amount` to validator
    /// Issue `amount` / exchange_rate for the user.
    Bond {},
    // BondRewards {},
    // Send back unbonded coin to the user
    // WithdrawUnbonded {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
    #[returns(State)]
    State {},
    #[returns(Parameters)]
    Parameters {},
    #[returns(ValidatorsRegistry)]
    Validators {},
}
