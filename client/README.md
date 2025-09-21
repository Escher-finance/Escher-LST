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


## How to run

> yarn install

> yarn run dev


### CURL RELAYER

To send to sepolia need to run curl relayer

### BABYLON to SEPOLIA

> curl -X POST "95.217.11.125:7177/enqueue"   -H "Content-Type: application/json"   -d '{"@type":"call","@value":{"@type":"plugin","@value":{"plugin":"voyager-plugin-packet-index","message":{"@type":"make_packet_event","@value":{"chain_id":"bbn-test-5","channel_id":1,"packet_hash":"replacethis"}}}}}'

### BABYLON TO HOLESKY

> curl -X POST "95.217.11.125:7177/enqueue"   -H "Content-Type: application/json"   -d '{"@type":"call","@value":{"@type":"plugin","@value":{"plugin":"voyager-plugin-packet-index","message":{"@type":"make_packet_event","@value":{"chain_id":"bbn-test-5","channel_id":2,"packet_hash":"replacethis"}}}}}'

### UNION TO SEPOLIA

> curl -X POST "95.217.11.125:7177/enqueue"   -H "Content-Type: application/json"   -d '{"@type":"call","@value":{"@type":"plugin","@value":{"plugin":"voyager-plugin-packet-index","message":{"@type":"make_packet_event","@value":{"chain_id":"union-testnet-10","channel_id":1,"packet_hash":"replacethis"}}}}}'

