use cosmwasm_std::{Decimal, Uint128};

use crate::state::*;

#[test]
fn test_update_exchange_rate_should_never_yield_zero() {
    let decimal_zero = Decimal::zero();

    let mut state = State {
        exchange_rate: decimal_zero,
        total_bond_amount: Uint128::zero(),
        total_supply: Uint128::zero(),

        total_delegated_amount: Uint128::default(),
        bond_counter: u64::default(),
        last_bond_time: u64::default(),
    };

    // If `total_bond_amount` and `total_supply` are zero
    state.total_bond_amount = Uint128::zero();
    state.total_supply = Uint128::zero();
    state.update_exchange_rate();
    assert_ne!(state.exchange_rate, decimal_zero);

    // If only `total_bond_amount` is zero
    state.total_bond_amount = Uint128::zero();
    state.total_supply = Uint128::one();
    state.update_exchange_rate();
    assert_ne!(state.exchange_rate, decimal_zero);

    // If only `total_supply` is zero
    state.total_bond_amount = Uint128::one();
    state.total_supply = Uint128::zero();
    state.update_exchange_rate();
    assert_ne!(state.exchange_rate, decimal_zero);
}
