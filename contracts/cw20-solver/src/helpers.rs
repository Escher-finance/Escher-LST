use cosmwasm_std::{DepsMut, StdResult, Uint128};

use crate::ContractError;

// Internal unchecked `mint`
pub fn _mint(deps: DepsMut, recipient: String, amount: Uint128) -> Result<(), ContractError> {
    let mut config = cw20_base::state::TOKEN_INFO.load(deps.storage)?;

    // update supply and enforce cap
    config.total_supply += amount;
    if let Some(limit) = config.get_cap() {
        if config.total_supply > limit {
            return Err(ContractError::Cw20Error(
                cw20_base::ContractError::CannotExceedCap {},
            ));
        }
    }
    cw20_base::state::TOKEN_INFO.save(deps.storage, &config)?;

    // add amount to recipient balance
    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    cw20_base::state::BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;
    Ok(())
}
