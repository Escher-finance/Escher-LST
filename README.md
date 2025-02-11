## EVM Union Liquid Staking

Liquid staking components for Cosmos/Cosmwasm based contract.

### Architecture

Please see this [Architecture](docs/architecture.md)

## Components:

### EVM/Bera (Solidity)

| Component                                   | Description                                          | Language(s)           |
| ------------------------------------------- | ---------------------------------------------------- | --------------------- |
| [ICA Controller Factory](https://github.com/Escher-finance/evm-ibc-apps/blob/main/contracts/ica/src/ICAControllerFactory.sol)                       | Solidity Factory contract to create new ICA Controller contract  | Solidity                  |
| [ICA Controller Contract](https://github.com/Escher-finance/evm-ibc-apps/blob/main/contracts/ica/src/ICAController.sol)                      | Solidity Factory contract to create new ICA and send messages to ICA  | Solidity                  |



### Cosmos/Cosmwasm


| Component                                   | Description                                                          | Language(s)                                         |
| ------------------------------------------- | -------------------------------------------------------------------- | ----------------------------------------------------|
| [Liquid Staking](https://github.com/Escher-finance/evm-union-liquid-staking/tree/main/contracts/liquidstaking)     | Cosmwasm Liquid Staking Contract           | Rust   |
| [Reward Contract](https://github.com/Escher-finance/evm-union-liquid-staking/tree/main/contracts/rewards)          | Reward contract to receive & split reward  | Rust    |
| [UCS30 Contract](https://github.com/unionlabs/union/tree/main/cosmwasm/ibc-union/app/ucs03-zkgm)         | UCS30 Contract to transfer token | Rust    |
| [CW20 Token Minter](https://github.com/unionlabs/union/tree/main/cosmwasm/cw20-token-minter)         | CW20 Contract to handle cw20 token mint and burn  | Rust    |


### Backend

| Component                                   | Description                                                          | Language(s)                                         |
| ------------------------------------------- | -------------------------------------------------------------------- | ----------------------------------------------------|
| [Liquid Staking Backend service](https://github.com/Escher-finance/liquid-staking-service)     | Cosmwasm Liquid Staking Contract           | Node/TS   |


#### Create Denom

TODO