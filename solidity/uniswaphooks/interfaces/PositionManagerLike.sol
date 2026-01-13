// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

/// Minimal types and interface to mirror the Uniswap v4 PositionManager flow.
/// This is a scaffold for local testing and encoding actions/params.
library PMTypes {
    struct PoolKey {
        address currency0;
        address currency1;
        uint24 fee; // optional; keep for completeness
        address hooks; // hook address if any
    }
}

interface IPositionManagerLike {
    /// @notice Modify liquidities via an encoded actions+params payload.
    /// @param actionsAndParams abi.encode(bytes actionsPacked, bytes[] params)
    /// @param deadline timestamp after which call should fail
    function modifyLiquidities(bytes calldata actionsAndParams, uint256 deadline) external payable;

    /// Optional: when manager already unlocked
    function modifyLiquiditiesWithoutUnlock(bytes calldata actionsAndParams) external payable;
}

