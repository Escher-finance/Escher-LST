use crate::helpers::{compute_mint_amount, compute_unbond_amount};

// Basic test - based on figures from excalidraw
#[test]
fn test_compute_mint_amount() {
    let total_native_token = 2_000_000_000;
    let total_liquid_stake_token = 1_800_000_000;
    let native_to_stake = 100_000_000;
    let mint_amount = compute_mint_amount(
        total_native_token,
        total_liquid_stake_token,
        native_to_stake,
    );

    assert_eq!(mint_amount, 90_000_000);
}

// Basic test - based on figures from excalidraw
#[test]
fn test_compute_unbond_amount() {
    let total_native_token = 2_000_000_000;
    let total_liquid_stake_token = 1_800_000_000;
    let batch_unstake = 90_000_000;
    let unbond_amount =
        compute_unbond_amount(total_native_token, total_liquid_stake_token, batch_unstake);

    assert_eq!(unbond_amount, 100_000_000);
}
