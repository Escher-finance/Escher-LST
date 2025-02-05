use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_json_binary, Addr, Coin, CosmosMsg, Decimal, StdResult, Uint128, WasmMsg};

use crate::msg::ExecuteLstMsg;

const DECIMAL_FRACTIONAL: Uint128 = Uint128::new(1_000_000_000_000_000_000u128);

/// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct LstTemplateContract(pub Addr);

impl LstTemplateContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteLstMsg>>(&self, msg: T, funds: Vec<Coin>) -> StdResult<CosmosMsg> {
        let msg = to_json_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds,
        }
        .into())
    }
}

pub fn split_revenue(amount: Uint128, fee_rate: Decimal, denom: String) -> (Coin, Coin) {
    println!("{:?}", Decimal::one().atomics());
    let decimal_fract = Decimal::new(DECIMAL_FRACTIONAL * DECIMAL_FRACTIONAL);
    let fract = (fee_rate * decimal_fract).to_uint_ceil();
    let fee_amount =
        Decimal::from_ratio(fract * amount, Uint128::from(DECIMAL_FRACTIONAL)).to_uint_floor();
    let redelegate_amount = amount - fee_amount;

    (
        Coin {
            amount: redelegate_amount,
            denom: denom.clone(),
        },
        Coin {
            amount: fee_amount,
            denom: denom.clone(),
        },
    )
}
