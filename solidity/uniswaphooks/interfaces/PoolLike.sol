// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

/// Minimal pool-like interface exposing current price and swap entrypoint.
/// Replace with real v4 interfaces when integrating.
interface IPoolLike {
    /// @notice Returns the current sqrt price Q64.96
    function sqrtPriceX96() external view returns (uint160);

    /// @notice Perform a swap in the pool
    /// @dev Signature is intentionally generic to keep this scaffold unopinionated.
    function swap(address recipient, bool zeroForOne, int256 amountSpecified, bytes calldata data)
        external
        returns (int256 amount0, int256 amount1);
}

/// Optional lightweight swapper that the hook can call to perform token transfers/approvals
/// prior to calling the pool.
interface ISwapper {
    /// @notice Execute the actual swap against the target pool.
    /// @param pool The pool to swap on
    /// @param zeroForOne Direction
    /// @param amountSpecified Signed amount (positive for exact in, negative for exact out)
    /// @param data Arbitrary data forwarded to pool callback mechanics
    function executeSwap(IPoolLike pool, bool zeroForOne, int256 amountSpecified, bytes calldata data)
        external
        returns (int256 amount0, int256 amount1);
}

