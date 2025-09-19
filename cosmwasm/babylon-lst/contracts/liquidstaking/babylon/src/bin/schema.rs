use cosmwasm_schema::write_api;
use liquidstaking_babylon::msg::{InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
    }
}
