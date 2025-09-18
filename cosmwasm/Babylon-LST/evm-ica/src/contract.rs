#[cfg(not(feature = "library"))]
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, entry_point};

// use cw2::set_contract_version;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:evm-ica";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cw_serde]
pub struct IcaPacketData {
    /// Type defines a classification of message issued from a controller
    /// chain to its associated interchain accounts host.
    ///
    /// There are two types of messages:
    /// * `0 (Unspecified)`: Default zero value enumeration. (Returns an error in host).
    /// * `1 (ExecuteTx)`: Execute a transaction on an interchain accounts host chain.
    ///
    /// `r#type` is used to avoid the reserved keyword `type`.
    #[serde(rename = "type")]
    pub r#type: u32,
    /// Data is the raw transaction data that will be sent to the interchain accounts host.
    /// Currently, the host only supports json (or proto) serialized Any messages.
    pub data: Vec<u8>,
    /// Memo is an optional field that can be used to attach a memo to a transaction.
    /// It is also caught by some ibc middleware to perform additional actions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

#[cfg(test)]
mod tests {
    use cosmos_sdk_proto::{
        ibc::applications::interchain_accounts::v1::InterchainAccountPacketData, traits::Message,
    };

    #[test]
    fn test_bytes() {
        let packet: &[u8] = &[
            8, 1, 18, 161, 1, 10, 158, 1, 10, 28, 47, 99, 111, 115, 109, 111, 115, 46, 98, 97, 110,
            107, 46, 118, 49, 98, 101, 116, 97, 49, 46, 77, 115, 103, 83, 101, 110, 100, 18, 126,
            10, 64, 117, 110, 105, 111, 110, 49, 104, 51, 104, 116, 121, 57, 56, 107, 57, 120, 103,
            106, 53, 119, 102, 102, 108, 117, 119, 54, 53, 97, 101, 121, 112, 103, 109, 113, 56,
            101, 57, 97, 50, 104, 101, 56, 56, 113, 115, 116, 53, 110, 101, 97, 104, 120, 121, 101,
            112, 117, 102, 115, 122, 102, 122, 113, 108, 112, 18, 44, 117, 110, 105, 111, 110, 49,
            118, 110, 103, 108, 104, 101, 119, 102, 51, 119, 54, 54, 99, 113, 117, 121, 54, 104,
            114, 55, 117, 114, 106, 118, 51, 53, 56, 57, 115, 114, 104, 101, 97, 109, 112, 122, 52,
            50, 26, 12, 10, 4, 109, 117, 110, 111, 18, 4, 49, 48, 48, 48, 26, 8, 98, 121, 32, 110,
            111, 109, 111, 115,
        ];

        let acc = InterchainAccountPacketData::decode(packet);
        // let ica_packet_data: Result<IcaPacketData, StdError> = from_json(bin);
        println!("{:?}", acc.unwrap());
    }
}
