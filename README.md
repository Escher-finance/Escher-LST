## Escher Liquid Staking contract

Liquid staking components for Cosmos/CosmWasm-based contracts.

Escher currently supports two contract versions:

- Babylon Chain:
  [Babylon Liquid Staking Contract](https://github.com/Escher-finance/cw-liquid-staking/tree/babylon/contracts/liquidstaking/liquidstaking-babylon)

- Union Chain: Separate implementation for the Union chain (you can see in the
  rest of the page).
  [Union Liquid Staking Contract](https://github.com/Escher-finance/cw-liquid-staking/tree/babylon/contracts/liquidstaking/liquidstaking-union)

Key Differences:

- On the Babylon chain, integration is local. This means interactions with the
  contract are limited to within the Babylon chain, and potentially through IBC
  connections.

- Reward management on Babylon is directly embedded within the staking contract,
  while on Union, rewards are managed by a separate contract. This design
  decision is due to Babylon's limitation that prevents changing the withdrawal
  reward address.

Babylon Contract Details:

- The Babylon contract includes optimization mechanisms to maintain validator
  balance. Every 360 blocks, the contract automatically executes staking,
  unstaking, delegation, collect reward, operations to ensure stable
  performance.

### Architecture

- For liquid staking contract implementation, we have 2 versions, one with
  separate reward contract and the other one is without. Please see this
  [Architecture](docs/architecture.md)

## Components:

### Cosmos/Cosmwasm

| Component                                                                                                      | Description                                      | Language(s) |
| -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | ----------- |
| [Liquid Staking](https://github.com/Escher-finance/evm-union-liquid-staking/tree/main/contracts/liquidstaking) | Cosmwasm Liquid Staking Contract                 | Rust        |
| [Reward Contract](https://github.com/Escher-finance/evm-union-liquid-staking/tree/main/contracts/rewards)      | Reward contract to receive & split reward        | Rust        |
| [CW20 Token Minter](https://github.com/unionlabs/union/tree/main/cosmwasm/cw20-token-minter)                   | CW20 Contract to handle cw20 token mint and burn | Rust        |
| [UCS30 Contract](https://github.com/unionlabs/union/tree/main/cosmwasm/ibc-union/app/ucs03-zkgm)               | UCS30 Contract to transfer token to other chain  | Rust        |

### Backend

| Component                                                                                  | Description                                                                                              | Language(s) |
| ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------- | ----------- |
| [Liquid Staking Backend service](https://github.com/Escher-finance/liquid-staking-service) | Liquid Staking Backend Job Service to do process rewards and automatic transfer after unbonding complete | Node/TS     |

#### Indexer

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
$ nix build .#liquidstaking-solo.release
$ ls -alhL result 
-r-xr-xr-x 1 root root 553K Jan  1  1970 result
$ nix run guthub:unionlabs/union#embed-commit-verifier -- extract result
1ad4854a1e1d2cbce71c6b064a190ebf1662f947 # the commit has of the repo when this package was built
```
