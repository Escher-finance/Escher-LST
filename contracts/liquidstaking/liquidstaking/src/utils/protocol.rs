use crate::msg::Ucs03ExecuteMsg;
use crate::utils::delegation::DEFAULT_TIMEOUT_TIMESTAMP_OFFSET;
use crate::ContractError;
use cosmwasm_std::{to_json_binary, Coin, Timestamp, Uint128, Uint256, WasmMsg};
use unionlabs_primitives::{Bytes, H256};

pub fn ucs03_transfer(
    time: Timestamp,
    ucs03_contract_addr: String,
    channel_id: u32,
    receiver: Bytes,
    base_token: String,
    base_amount: Uint128,
    quote_token: Bytes,
    quote_amount: Uint256,
    funds: Vec<Coin>,
    salt: H256,
) -> Result<WasmMsg, ContractError> {
    let timeout = time.plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET).nanos();

    let relay_transfer_msg: Ucs03ExecuteMsg = Ucs03ExecuteMsg::Transfer {
        channel_id,
        receiver,
        base_token,
        base_amount,
        quote_token,
        quote_amount,
        timeout_height: 0,
        timeout_timestamp: timeout,
        salt,
    };

    let transfer_relay_msg = to_json_binary(&relay_transfer_msg)?;

    return Ok(WasmMsg::Execute {
        contract_addr: ucs03_contract_addr,
        msg: transfer_relay_msg,
        funds,
    });
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::from_json;

    use super::*;

    #[test]
    fn test_ucs03_transfer() {
        let ucs03_contract_addr = "ucs03".to_string();
        let ucs03_funds = Vec::from([Coin::new(Uint128::new(100), "denom")]);
        let result = ucs03_transfer(
            Timestamp::default(),
            ucs03_contract_addr.clone(),
            u32::default(),
            Bytes::default(),
            String::default(),
            Uint128::default(),
            Bytes::default(),
            Uint256::default(),
            ucs03_funds.clone(),
            H256::default(),
        );
        if let WasmMsg::Execute {
            contract_addr,
            msg,
            funds,
        } = result.unwrap()
        {
            assert_eq!(contract_addr, ucs03_contract_addr);
            assert_eq!(funds, ucs03_funds);
            from_json::<Ucs03ExecuteMsg>(msg).unwrap();
        } else {
            panic!("not WasmMsg::Execute");
        }
    }
}
