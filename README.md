## EVM Union Liquid Staking

Liquid staking components for Cosmos/Cosmwasm based contract.

#### Create Denom

export RPC=https://rpc.testnet-8.union.build:443
export SENDER=isak
export HOME=/home/isak/.union

#### Tokenfactory

uniond tx  --node $RPC --gas auto --gas-adjustment 1.4 --from $SENDER --home $HOME tokenfactory create-denom stmomo

uniond --node $RPC query tokenfactory denoms-from-creator  union1vnglhewf3w66cquy6hr7urjv3589srheampz42 

uniond --node $RPC query bank balances union1vnglhewf3w66cquy6hr7urjv3589srheampz42

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
| Liquid Staking Token                        |                                                                     |  |
| [TokenFactory](https://github.com/unionlabs/union/blob/dcea7c0e4e4da816c9313c1fd1d8b1e53e83e086/uniond/x/tokenfactory/README.md)         | Union module to create new token   | Rust    |
| [CW20 Contract](https://github.com/unionlabs/union/blob/dcea7c0e4e4da816c9313c1fd1d8b1e53e83e086/uniond/x/tokenfactory/README.md)         | CW20 Contract to create new token  | Rust    |


### Backend

| Component                                   | Description                                                          | Language(s)                                         |
| ------------------------------------------- | -------------------------------------------------------------------- | ----------------------------------------------------|
| [Native,Liquid Staking Backend service](https://github.com/Escher-finance/liquid-staking-service)     | Cosmwasm Liquid Staking Contract           | Node/TS   |
| [Process Rewards](https://github.com/Escher-finance/evm-union-liquid-staking/blob/5aa85035f49d4126aa4d68c144d34c25a29fc398/backend/src/index.ts#L35)     | Nodejs script to handle process rewards | Node/TS   |
| [Process Unbonding](https://github.com/Escher-finance/evm-union-liquid-staking/blob/5aa85035f49d4126aa4d68c144d34c25a29fc398/backend/src/index.ts#L9)    | Nodejs script to handle process unbonding            | Node/TS   |


