## Escher Liquid Staking contract

Liquid staking components for Cosmos/CosmWasm-based contracts.

Escher currently supports two contract versions:

- Babylon Chain:
  [Babylon Liquid Staking Contract](https://github.com/Escher-finance/cw-liquid-staking/tree/babylon/contracts/liquidstaking/liquidstaking-solo)

- Union Chain: Separate implementation for the Union chain (you can see in the
  rest of the page).

Key Differences:

- On the Babylon chain, integration is local. This means interactions with the
  contract are limited to within the Babylon chain, and potentially through IBC
  connections.

- Reward management on Babylon is directly embedded within the staking contract,
  while on Union, rewards are managed by a separate contract. This design
  decision is due to Babylon's limitation that prevents changing the withdrawal
  reward address.

Babylon Contract Details:

- The Babylon contract includes optimization mechanisms to maintain validator
  balance. Every 360 blocks, the contract automatically executes staking,
  unstaking, delegation, collect reward, operations to ensure stable
  performance.

### Architecture

- For liquid staking contract implementation, we have 2 versions, one with
  separate reward contract and the other one is without. Please see this
  [Architecture](docs/architecture.md)

## Components:

### Cosmos/Cosmwasm

| Component                                                                                                      | Description                                      | Language(s) |
| -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | ----------- |
| [Liquid Staking](https://github.com/Escher-finance/evm-union-liquid-staking/tree/main/contracts/liquidstaking) | Cosmwasm Liquid Staking Contract                 | Rust        |
| [Reward Contract](https://github.com/Escher-finance/evm-union-liquid-staking/tree/main/contracts/rewards)      | Reward contract to receive & split reward        | Rust        |
| [CW20 Token Minter](https://github.com/unionlabs/union/tree/main/cosmwasm/cw20-token-minter)                   | CW20 Contract to handle cw20 token mint and burn | Rust        |
| [UCS30 Contract](https://github.com/unionlabs/union/tree/main/cosmwasm/ibc-union/app/ucs03-zkgm)               | UCS30 Contract to transfer token to other chain  | Rust        |

### Backend

| Component                                                                                  | Description                                                                                              | Language(s) |
| ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------- | ----------- |
| [Liquid Staking Backend service](https://github.com/Escher-finance/liquid-staking-service) | Liquid Staking Backend Job Service to do process rewards and automatic transfer after unbonding complete | Node/TS     |

#### Indexer

| Component                                                   | Description     | Language(s) |
| ----------------------------------------------------------- | --------------- | ----------- |
| [Indexer](https://github.com/Escher-finance/cosmos-indexer) | Cosmwos Indexer | Rust        |
