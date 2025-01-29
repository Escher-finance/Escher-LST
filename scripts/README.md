# LST Contract Deployment Scripts

1. To run the script to upload and instantiate contracts we need these data in env vars. Set these values and put in .env file

- MNEMONIC="mnemonic of account"
- CW20_WASM_PATH="cw20 contract wasm absolute file path"
- LST_WASM_PATH="liquid staking contract wasm absolute file path"
- REWARD_WASM_PATH="reward contract wasm absolute file path"



2. Prepare these data as env vars and set in src/config.ts

- chainId
- chainName
- coinDenom
- lstCoinDenom
- lstCoinSymbol
- rpc
- rest
- ucs03Channel
- ucs03RelayContract
- feeRate
- revenueReceiver
- validator

3. Run setup script

```
npm install
npm run start
```