# Liquid Staking Contract

Liquid Staking Contract that enables liquid staking on union chain.


### Build contract:

> RUSTFLAGS="-C link-arg=-s" cargo build --release --lib --target=wasm32-unknown-unknown
> sudo wasm-opt -Os --signext-lowering "target/wasm32-unknown-unknown/release/liquidstaking.wasm" -o "artifacts/liquidstaking.wasm"

The wasm file output will be on artifacts folder.

#### Build for Non Union Chain

We can use this contract for Non Union chain (utilizing CW20)
[Read more](./e2e/interchaintest/README.md)

### Prequisites:

In order to make this liquid contract works, this contract will require tokenfactory denom, so we need to [create new denom using tokenfactory](https://github.com/unionlabs/union/tree/main/uniond/x/tokenfactory) and this contract need be set as admin of liquid staking denom in Union chain

Example: 

> uniond --node $RPC --gas auto --gas-adjustment 1.5 --from isak --home /home/isak/.union  tx tokenfactory change-admin  factory/union1vnglhewf3w66cquy6hr7urjv3589srheampz42/stmomo union1x70fmdv965fj6hm4lmyudxyphl6j9vweukmc3fxja3mamgqrup6qf9mv3x



### E2E Test

This contract has E2E Test

[Read more](./e2e/interchaintest/README.md)
