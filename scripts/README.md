# Deployment Scripts

1. Run upload liquid staking and reward script

Get the code id of those contracts

2. Prepare these data as env vars and put in .env file

- underlying_coin_denom
- validators (put as comma separated values)
- liquidstaking_denom
- ucs01_channel
- ucs01_relay_contract
- revenue_receiver
- unbonding_time
- cw20_address

2. Run instantiate liquid staking script