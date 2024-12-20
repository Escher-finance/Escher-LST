# Reward contract

This contract will receive delegator reward and can split reward to call redelegate to lst contract and send the fee to revenue receiver

To instantiate need these config:

- lst_contract_address
- revenue_receiver
- fee_rate
- coin_denom

## Build wasm

> cargo build

> cargo run-script optimize