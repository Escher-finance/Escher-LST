# Uniswap v4 Limit Order Hook (scaffold)

This module scaffolds a basic "limit order" hook-like contract intended to work with Uniswap v4-style pools. It is not wired to real v4 core contracts; instead, it uses small, self-contained interfaces so you can integrate with actual v4 deployments later.

What it does:
- Register simple limit orders with a trigger price in sqrtPriceX96.
- Cancel orders before execution.
- Keeper-style execution: when the pool price crosses the trigger, perform a swap via a pluggable `ISwapper` interface.

References:
- Position Manager guide and action batching design in the Uniswap v4 docs: `https://docs.uniswap.org/contracts/v4/guides/position-manager`.

Notes:
- This is a minimal scaffold. A production v4 Hook would implement the official v4 hook interfaces and be registered on pool creation; here we keep dependencies light so you can iterate quickly and plug in the real interfaces later.

Directory:
- `interfaces/` – tiny v4-like interfaces used by the hook.
- `libraries/Actions.sol` – action constants (for reference; not strictly required by this scaffold, but handy if you expand to Position Manager batching per docs).
- `LimitOrderHook.sol` – main contract.


