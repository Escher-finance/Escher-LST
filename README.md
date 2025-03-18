## Escher Liquid Staking

Liquid staking components for Cosmos/Cosmwasm based contract.

### Architecture

For liquid staking contract implementation, we have 2 versions, one with separate reward contract and the other one is without.
Please see this [Architecture](docs/architecture.md)

## Components:

### Cosmos/Cosmwasm

| Component                                   | Description                                                          | Language(s)                                         |
| ------------------------------------------- | -------------------------------------------------------------------- | ----------------------------------------------------|
| [Liquid Staking with Reward](https://github.com/Escher-finance/evm-union-liquid-staking/tree/main/contracts/liquidstaking/liquidstaking)     | Cosmwasm Liquid Staking Contract for Union          | Rust   |
| [Liquid Staking no Reward](https://github.com/Escher-finance/evm-union-liquid-staking/tree/babylon/contracts/liquidstaking/liquidstaking-solo)     | Cosmwasm Liquid Staking Contract without Reward contract for Babylon         | Rust   |
| [Reward Contract](https://github.com/Escher-finance/evm-union-liquid-staking/tree/main/contracts/rewards)          | Reward contract to receive & split reward  | Rust    |
| [CW20 Token Minter](https://github.com/unionlabs/union/tree/main/cosmwasm/cw20-token-minter)         | CW20 Contract to handle cw20 token mint and burn  | Rust    |
| [UCS30 Contract](https://github.com/unionlabs/union/tree/main/cosmwasm/ibc-union/app/ucs03-zkgm)         | UCS30 Contract to transfer token to other chain | Rust    |

### Backend

| Component                                   | Description                                                          | Language(s)                                         |
| ------------------------------------------- | -------------------------------------------------------------------- | ----------------------------------------------------|
| [Liquid Staking Backend service](https://github.com/Escher-finance/liquid-staking-service)     | Liquid Staking Backend Job Service to do process rewards and automatic transfer after unbonding complete           | Node/TS   |


#### Indexer

| Component                                   | Description                                                          | Language(s)                                         |
| ------------------------------------------- | -------------------------------------------------------------------- | ----------------------------------------------------|
| [Indexer](https://github.com/Escher-finance/cosmos-indexer)     | Cosmwos Indexer       | Rust  |


