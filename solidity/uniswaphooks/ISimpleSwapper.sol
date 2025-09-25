// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {IPoolLike, ISwapper} from "./interfaces/PoolLike.sol";

/// Minimal swapper that forwards swaps to the pool. In production you would
/// handle token approvals, callbacks, and accounting here.
contract SimpleSwapper is ISwapper {
    function executeSwap(
        IPoolLike pool,
        bool zeroForOne,
        int256 amountSpecified,
        bytes calldata data
    ) external override returns (int256 amount0, int256 amount1) {
        return pool.swap(msg.sender, zeroForOne, amountSpecified, data);
    }
}


