# uh oh

https://app.union.build/explorer/packets/0x1ad59035d64db291c4c4288059895cd3588ce45ac3adabcc7fc12e25131ba579

query on ebaby the current balance of the staking hub:

```sh
nix run .#babylond -- query wasm contract-state smart bbn1s7jzz7cyuqmy5xpr07yepka5ngktexsferu2cr4xeww897ftj77sv30f5s '{"balance":{"address":"bbn1m7zr5jw4k9z22r9ajggf4ucalwy7uxvu9gkw6tnsmv42lvjpkwasagek5g"}}' --node https://babylon-rpc.polkachu.com
```

```yaml
data:
  balance: "4660566803"
```

query on the staking hub the current batch:

```sh
nix run .#babylond -- query wasm contract-state smart bbn1m7zr5jw4k9z22r9ajggf4ucalwy7uxvu9gkw6tnsmv42lvjpkwasagek5g '{"batch":{}}' --node https://babylon-rpc.polkachu.com
```

```yaml
data:
- expected_native_unstaked: null
  id: 459
  next_batch_action_time: 1756603198
  received_native_unstaked: null
  status: pending
  total_liquid_stake: "4660596803"
  unbond_records_count: 4
  ```

note that 4660596803 - 4660566803 = 30000, which is the amount we sent in the packet
