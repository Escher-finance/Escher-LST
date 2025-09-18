# EVM Union LST Hub

## Usage

### Build

```shell
$ forge build
```

### Test

```shell
$ forge test
```

To test with log:

```shell
$ forge test -vv
```

### Test coverage

```shell
$ forge coverage --ir-minimum
```

### Deploy

```shell
$ forge script script/DeployLst.s.sol:DeployLstScript --rpc-url {chain} --account {account}
```
