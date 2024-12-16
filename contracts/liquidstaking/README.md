# Liquid Staking Contract

Liquid Staking Contract that enables liquid staking on union chain.

### Step before using

In order to make this liquid contract works, this contract need to be set as admin of
liquid staking denom in Union chain

Example: 

> uniond --node $RPC --gas auto --gas-adjustment 1.5 --from isak --home /home/isak/.union  tx tokenfactory change-admin  factory/union1vnglhewf3w66cquy6hr7urjv3589srheampz42/stmomo union1x70fmdv965fj6hm4lmyudxyphl6j9vweukmc3fxja3mamgqrup6qf9mv3x

Build contract:

> cargo run-script optimize

The output will be on artifacts folder.

### E2E Test

This contract has E2E Test

[Read more](./e2e/interchaintest/README.md)