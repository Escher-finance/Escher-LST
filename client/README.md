# Frontend Development App

### Config

1. Menu config
config/site.ts

2. Network config
config/networks.config.ts


### STEPS to run Non union/Ignite chain client demo app:

1. Prepare the cw20 wasm contract and liquid staking contract wasm file

2. Upload those contract and get code id

3. Instantiate cw20 contract and set minter to user

4. Instantiate liquid staking contract 

5. Set minter of cw20 contract to liquid staking contract address

6. Put the address to config
