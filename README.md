## EVM Union Liquid Staking

Liquid staking project from EVM/Berachain to Union


### Create Denom

export RPC=https://rpc.testnet-8.union.build:443
export SENDER=isak
export HOME=/home/isak/.union



### Tokenfactory

uniond tx  --node $RPC --gas auto --gas-adjustment 1.4 --from $SENDER --home $HOME tokenfactory create-denom stmomo

uniond --node $RPC query tokenfactory denoms-from-creator  union1vnglhewf3w66cquy6hr7urjv3589srheampz42 

uniond --node $RPC query bank balances union1vnglhewf3w66cquy6hr7urjv3589srheampz42