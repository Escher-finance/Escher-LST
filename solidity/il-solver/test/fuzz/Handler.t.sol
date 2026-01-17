//SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {IlSolverMath} from "../../src/core/EscherMath.sol";

contract Handler is Test {
    // Max values for bounding
    uint256 public constant MAX_COLLATERAL_AMOUNT = type(uint96).max;
    uint256 public constant MAX_BORROWED_AMOUNT = type(uint96).max;
    uint256 public constant MAX_BORROW_AMOUNT_USD = type(uint96).max;
    uint256 public constant MAX_LTV = 98e16; // 98%
    uint256 public constant MIN_LTV = 21e15; // 2.1% (above safety factor)

    uint8 constant DEFAULT_DECIMALS = 18;

    // Track calls for invariants
    uint256 public callCount;
    uint256 public successfulCalls;
    uint256 public failedCalls;

    // Ghost variables to track state
    uint256 public lastIterations;
    uint256 public lastTotalBorrowed;
    bool public lastIsEnough;

    function hedgingLoop(uint256 collateralAmount, uint256 borrowedAmount, uint256 borrowAmountUSD, uint256 ltv)
        public
    {
        callCount++;

        // Bound inputs to valid and reasonable ranges
        collateralAmount = bound(collateralAmount, 100 * 1e18, 10000000 * 1e18); // 100-10M USD
        borrowedAmount = bound(borrowedAmount, 1 * 1e16, 10000 * 1e18); // 0.01-10k tokens
        borrowAmountUSD = bound(borrowAmountUSD, 10 * 1e18, 100000 * 1e18); // $10-$100k
        ltv = bound(ltv, MIN_LTV, MAX_LTV);

        (uint256 iterations, bool isEnough, uint256 totalBorrowed, uint256 ltvUsed,) = IlSolverMath.hedgingLoop(
            collateralAmount, borrowedAmount, borrowAmountUSD, DEFAULT_DECIMALS, DEFAULT_DECIMALS, ltv
        );
        assertEq(isEnough, true);
        lastIterations = iterations;
        lastTotalBorrowed = totalBorrowed;
        lastIsEnough = isEnough;
    }

    function calculateCollateralAmount(uint256 borrowedAmount, uint256 borrowAmountUSD, uint256 ltv) public {
        callCount++;

        // Bound inputs to valid and reasonable ranges
        borrowedAmount = bound(borrowedAmount, 1 * 1e16, 10000 * 1e18); // 0.01-10k tokens
        borrowAmountUSD = bound(borrowAmountUSD, 10 * 1e18, 100000 * 1e18); // $10-$100k
        ltv = bound(ltv, MIN_LTV, MAX_LTV);

        (uint256 collateral,,,,,) = IlSolverMath.calculateCollateralAmount(
            borrowedAmount, borrowAmountUSD, DEFAULT_DECIMALS, DEFAULT_DECIMALS, ltv
        );
        (uint256 iterations, bool isEnough, uint256 totalBorrowed, uint256 ltvUsed,) = IlSolverMath.hedgingLoop(
            collateral, borrowedAmount, borrowAmountUSD, DEFAULT_DECIMALS, DEFAULT_DECIMALS, ltv
        );
        assertEq(isEnough, true);
    }
}
