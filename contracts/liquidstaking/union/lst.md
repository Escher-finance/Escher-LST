- logic reqs:

  - liquid staking entrypoint
  - liquid unstaking entrypoint
  - submit batch
    - apply all unstake requests in the batch (stake is immediate, not batched)
      - burn lst tokens
      - withdraw unstaked tokens
  - receive rewards
    - split rewards functionality?

- fungible counterparty configurations so that we can get the quote token for the destination channels (recipient_channel_id)

- since the staking is happening on the same chain that the lst is living on, there is no need for the complex receive_rewards. this logic will instead happen in ??!??!!?!??

- there will need to be a second contract that we call into that does the staking. this enables us to embed automatic staking logic into this secondary contract, but also means we don't need to implement this immediately (if we don't want to/ run out of time)

  - this contract can also for example automatically send to the fee recipient on reward receipt

# questions

- in liquidstaking-babylon, why are receive rewards and split rewards different entrypoints? can they not be the same?

# TODOs

- call addr_validate on all Addr types

# Changes

- liquid stake -> bond
- liquid unstake -> unbond
