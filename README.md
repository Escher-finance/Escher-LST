## Escher Liquid Staking contract

Liquid staking components for Cosmos/CosmWasm-based contracts.

- Babylon Chain:
  [Babylon Liquid Staking Contract](https://github.com/Escher-finance/Escher-LST/tree/main/cosmwasm/babylon-lst/contracts/liquidstaking/babylon)

### Babylon Contract Details:

The Babylon contract includes optimization mechanisms to maintain validator balance. 
Every 360 blocks, the contract automatically executes staking,  unstaking, delegation, collect reward, operations to ensure stable performance. 
Reward management on Babylon is directly embedded within the liquid staking contract.

### Architecture

- For liquid staking contract architecture implementation, please see:
  [Architecture](./docs/architecture.md)

## Components:

### Cosmos/Cosmwasm

| Component                                                                                                             | Description                                      | Language(s) |
| --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | ----------- |
| [Liquid Staking](https://github.com/Escher-finance/Escher-LST/tree/main/cosmwasm/babylon-lst/contracts/liquidstaking) | Cosmwasm Liquid Staking Contract                 | Rust        |
| [CW20 Token Minter](https://github.com/unionlabs/union/tree/main/cosmwasm/cw20-token-minter)                          | Union CW20 to handle cw20 token mint & burn      | Rust        |
| [UCS30 Zkgm](https://github.com/unionlabs/union/tree/main/cosmwasm/app/ucs03-zkgm)                                    | Union UCS30 Zkgm to transfer asset               | Rust        |

### Backend

| Component                                                                                  | Description                                                                                              | Language(s) |
| ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------- | ----------- |
| [Liquid Staking Backend service](https://github.com/Escher-finance/liquid-staking-service) | Liquid Staking Backend Job Service to do process rewards and automatic transfer after unbonding complete | Node/TS     |

### Indexer

| Component                                                   | Description     | Language(s) |
| ----------------------------------------------------------- | --------------- | ----------- |
| [Indexer](https://github.com/Escher-finance/cosmos-indexer) | Cosmwos Indexer | Rust        |

# Nix

This repo uses [nix](https://nixos.org/) for deterministic and reproducible builds and development environments.

If you are unfamiliar with nix, we recommend [zero to nix](https://zero-to-nix.com/).

To enter the dev shell for this repo, run:

```sh
nix develop
```

To format the repo:

```sh
nix fmt
```

To build a package:

```sh
nix build .#liquidstaking-babylon
```

The built artifacts can be found in `result`. All rust packages will have the commit hash embeded at build time, via [`embed-commit`](https://github.com/unionlabs/union/tree/main/lib/embed-commit). This is handled transparently by the crane builders from the union repo; the only requirement is that the `embed-commit` crate is added as a dependency (and the builds will fail if it is not a dependency):

```sh
$ nix build .#liquidstaking-babylon.release
$ ls -alhL result
-r-xr-xr-x 1 root root 553K Jan  1  1970 result
$ nix run guthub:unionlabs/union#embed-commit-verifier -- extract result
1ad4854a1e1d2cbce71c6b064a190ebf1662f947 # the commit has of the repo when this package was built
```
