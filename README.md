## Escher Liquid Staking

Liquid staking components for Cosmos/Cosmwasm based contract.

### Architecture

This liquid staking contract utilize other contracts like Reward contract to handle redelegate, split reward and also CW20 Contract for liquid staking token. In order to transfer to other chain, getting token from other chain and call from other chain, Escher utilizes UC03 relayer from Union.

For understanding the liquid staking flow, please see this [Architecture](docs/architecture.md)

### BABYLON version

To support [Babylon chain](https://babylonlabs.io/), we have another version without Reward Contract as Babylon chain doesn't support custom withdrawal address. Please check [Babylon](https://github.com/Escher-finance/cw-liquid-staking/tree/babylon)

## Components:

### Cosmos/Cosmwasm

| Component                                   | Description                                                          | Language(s)                                         |
| ------------------------------------------- | -------------------------------------------------------------------- | ----------------------------------------------------|
| [Liquid Staking](https://github.com/Escher-finance/evm-union-liquid-staking/tree/main/contracts/liquidstaking)     | Cosmwasm Liquid Staking Contract           | Rust   |
| [Reward Contract](https://github.com/Escher-finance/evm-union-liquid-staking/tree/main/contracts/rewards)          | Reward contract to receive, split reward, redelegate  | Rust    |
| [UCS30 Contract](https://github.com/unionlabs/union/tree/main/cosmwasm/ibc-union/app/ucs03-zkgm)         | UCS30 Contract to transfer token to other chain | Rust    |
| [CW20 Token Minter](https://github.com/unionlabs/union/tree/main/cosmwasm/cw20-token-minter)         | CW20 Contract to handle cw20 token mint and burn  | Rust    |


### Backend

| Component                                   | Description                                                          | Language(s)                                         |
| ------------------------------------------- | -------------------------------------------------------------------- | ----------------------------------------------------|
| [Liquid Staking Backend service](https://github.com/Escher-finance/liquid-staking-service)     | Cosmwasm Liquid Staking Contract           | Node/TS   |
| [Indexer](https://github.com/Escher-finance/escher-indexer)     | Union Escher Indexer       | Rust   |
