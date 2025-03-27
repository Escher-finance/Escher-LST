# Liquid Staking Contract

Liquid Staking Contract that enables liquid staking on union chain.


### Build contract:

```
RUSTFLAGS="-C link-arg=-s" cargo build --release --lib --target=wasm32-unknown-unknown

sudo wasm-opt -Os --signext-lowering "target/wasm32-unknown-unknown/release/liquidstaking.wasm" -o "artifacts/liquidstaking.wasm" 
```

The wasm file output will be on artifacts folder.

#### Build for Non Union Chain

We can use this contract for Non Union chain (utilizing CW20)
[Read more](./e2e/interchaintest/README.md)

### Prequisites:

In order to make this liquid contract works, this contract will require cw20 token minter set to this contract


### E2E Test

This contract has E2E Test

[Read more](./e2e/interchaintest/README.md)
