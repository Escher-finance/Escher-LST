# E2E Test

In order to run e2e test using interchaintest, this contract will utilize CW20 token as liquid staking token so it need specific cfg flag nonunion to be set.

This e2e test will test these functionalities:

- Bond
   The bond action will delegate to validators according to the weight configuration then send the cw20 token  to staker. The cw20 token is called lmuno

- Unbond
    Before unbond happen, the go test script will need to transfer lmuno token to contract via CW20 contract first

- Process Reward
    Process reward with withdraw reward and then restake and transfer 10% of the reward total to revenue receiver address

- Process Unbonding
    Process unbonding will transfer the native token (token) to user who did the unbond according to unbond record


### How to run e2e test

1. Make sure you are on e2e branch 
2. Build the contract wasm file first for CW20 and Liquid Staking Contract

To build wasm for Liquid Staking Contract:
Go to root directory of liquid staking contract then build wasm with rust flag cfg is set to nonunion

Before running this command, make sure you install wasm-opt first:
> cargo install wasm-opt

Then if u have wasm-opt already, run this command:

> RUSTFLAGS="-C link-arg=-s --cfg nonunion" cargo build --release --lib --target=wasm32-unknown-unknown
> wasm-opt -Os --signext-lowering "target/wasm32-unknown-unknown/release/evm_union_liquid_staking.wasm" -o "e2e/contracts/evm_union_liquid_staking.wasm"


3. Run the Go test in interchaintest folder

For first time, run go module installation first with this command:
> go mod tidy

If the modules are installed, we can run using this command:

> go test -v

Check the output log file in test_output.log


