# Backend for Liquid Staking

This backend script is used to process rewards and process unbonding and will be called by cron job script.

## Setup:

> Setup these env vars in .env:

```
RPC_URL=""

MNEMONIC=""

LST_CONTRACT_ADDRESS=""
```

Install npm

> npm install

Then to call process unconding use this command:

> npm run start --name process_unbonding
