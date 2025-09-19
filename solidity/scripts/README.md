# Foundry Scripts

See the respective script files to see all the `--sig` variants and understand
what each one does.

It is recommended to run all scripts with
`--account my-account --sender "$(cast w a --account my-account)"`, to make sure
`msg.sender` inside each script corresponds to the actual account running the
script and not the default Foundry account.

## `ZkgmTransfer.s.sol`

Makes a ZKGM TokenOrderV2 transfer of a single asset.

How to run:

```shell
forge script solidity/scripts/ZkgmTransfer.s.sol \
--sig 'run(address token, uint256 amount, uint32 channelId, string memory quoteToken, string memory receiver)' \
0xeeEEeeE98622c19Ea39Ea8827ae22Bbfc732671c 100 1 0xeeEEeeE98622c19Ea39Ea8827ae22Bbfc732671c 0x1285a2214319Eff512C5035933ac44E573738772 \
--rpc-url holesky --account escher-dev --sender "$(cast w a --account escher-dev)" --broadcast
```
