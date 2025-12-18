// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import {Test, console} from "forge-std/Test.sol";
import {IlSolverMath} from "../src/core/EscherMath.sol";
import {console} from "forge-std/console.sol";

// Wrapper contract to test library reverts properly
contract IlSolverMathWrapper {
    function hedgingLoop(
        uint256 collateralAmount,
        uint256 borrowedAmountNeeded,
        uint256 borrowAmountUSDPrice,
        uint256 ltv
    ) public pure returns (uint256, bool, uint256, uint256) {
        return IlSolverMath.hedgingLoop(collateralAmount, borrowedAmountNeeded, borrowAmountUSDPrice, ltv);
    }

    function calculateCollateralAmount(uint256 borrowedAmountNeeded, uint256 borrowAmountUSDPrice, uint256 ltv)
        public
        pure
        returns (uint256)
    {
        return IlSolverMath.calculateCollateralAmount(borrowedAmountNeeded, borrowAmountUSDPrice, ltv);
    }
}

contract IlSolverMathTest is Test {
    IlSolverMathWrapper wrapper;

    function setUp() public {
        wrapper = new IlSolverMathWrapper();
    }

    // --- sqrt tests ---

    // --- hedgingLoop tests ---

    function test_hedgingLoop_singleIterationEnough() public {
        // Simple case: one loop is enough to reach target
        uint256 collateralAmount = 2224 * 1e18;
        uint256 borrowedAmount = 1 * 1e18;
        uint256 borrowAmountUSD = 2000 * 1e18;
        uint256 ltv = 90e16; // 0.90 * 1e18

        (uint256 iterations, bool isEnough, uint256 totalBorrowedToken, uint256 ltvUsed) =
            IlSolverMath.hedgingLoop(collateralAmount, borrowedAmount, borrowAmountUSD, ltv);
        console.log("iterations (1e18 scale)", iterations);
        console.log("iterations (readable)", iterations / 1e18);
        console.log("collateralAmount", collateralAmount);
        console.log("isEnough", isEnough);
        console.log("totalBorrowedToken", totalBorrowedToken);
        console.log("ltvUsed", ltvUsed);

        // With 2224 USD, should need fractional iterations (less than 2)
        assertLt(iterations, 2 * 1e18); // Less than 2.0 iterations
        assertGt(iterations, 1 * 1e18); // More than 1.0 iteration
        assertEq(isEnough, true);
    }

    function testcheckcalculateCollateralAmountSingle() public {
        uint256 borrowedAmountNeeded = 1 * 1e18;
        uint256 borrowAmountUSDPrice = 2000 * 1e18;
        uint256 ltv = 90e16; // 0.90 * 1e18

        uint256 collateralAmountNeeded =
            IlSolverMath.calculateCollateralAmount(borrowedAmountNeeded, borrowAmountUSDPrice, ltv);
        console.log("collateralAmountNeeded", collateralAmountNeeded);
        assertEq(collateralAmountNeeded, 1136363636363636363636);
    }

    // --- Tests with different LTV values ---

    function test_hedgingLoop_highLTV() public {
        // Higher LTV (95%) - more leverage, fewer iterations needed
        uint256 collateralAmount = 1500 * 1e18;
        uint256 borrowedAmount = 5 * 1e18;
        uint256 borrowAmountUSD = 1000 * 1e18;
        uint256 ltv = 95e16; // 0.95 (93% after safety factor)

        (uint256 iterations, bool isEnough, uint256 totalBorrowedToken,) =
            IlSolverMath.hedgingLoop(collateralAmount, borrowedAmount, borrowAmountUSD, ltv);

        console.log("High LTV Test:");
        console.log("  iterations:", iterations / 1e16); // 2 decimals
        console.log("  isEnough:", isEnough);
        console.log("  totalBorrowed:", totalBorrowedToken / 1e18);

        assertTrue(isEnough);
        assertGe(totalBorrowedToken, borrowedAmount);
    }

    function test_hedgingLoop_lowLTV() public {
        // Lower LTV (70%) - less leverage, more iterations needed
        uint256 collateralAmount = 5000 * 1e18;
        uint256 borrowedAmount = 2 * 1e18;
        uint256 borrowAmountUSD = 2000 * 1e18;
        uint256 ltv = 70e16; // 0.70 (68% after safety factor)

        (uint256 iterations, bool isEnough, uint256 totalBorrowedToken,) =
            IlSolverMath.hedgingLoop(collateralAmount, borrowedAmount, borrowAmountUSD, ltv);

        console.log("Low LTV Test:");
        console.log("  iterations:", iterations / 1e16);
        console.log("  isEnough:", isEnough);
        console.log("  totalBorrowed:", totalBorrowedToken / 1e18);

        assertTrue(isEnough);
        assertGe(totalBorrowedToken, borrowedAmount);
    }

    // --- Tests with different token prices ---

    function test_calculateCollateral_highPrice() public {
        // Expensive token: $10,000 per token
        uint256 borrowedAmountNeeded = 1 * 1e18;
        uint256 borrowAmountUSDPrice = 10000 * 1e18;
        uint256 ltv = 90e16;

        uint256 collateralNeeded =
            IlSolverMath.calculateCollateralAmount(borrowedAmountNeeded, borrowAmountUSDPrice, ltv);

        console.log("High Price Test:");
        console.log("  collateral needed:", collateralNeeded / 1e18, "USD");

        // Should need more collateral for expensive tokens
        assertGt(collateralNeeded, 5000 * 1e18);
    }

    function test_calculateCollateral_lowPrice() public {
        // Cheap token: $1 per token
        uint256 borrowedAmountNeeded = 1000 * 1e18;
        uint256 borrowAmountUSDPrice = 1 * 1e18;
        uint256 ltv = 90e16;

        uint256 collateralNeeded =
            IlSolverMath.calculateCollateralAmount(borrowedAmountNeeded, borrowAmountUSDPrice, ltv);

        console.log("Low Price Test:");
        console.log("  collateral needed:", collateralNeeded / 1e18, "USD");

        // Should need reasonable collateral for cheap tokens
        assertGt(collateralNeeded, 500 * 1e18);
        assertLt(collateralNeeded, 1500 * 1e18);
    }

    // --- Large amount tests ---

    function test_calculateCollateral_largeAmount() public {
        // Large borrowing: 100 ETH worth $200,000
        uint256 borrowedAmountNeeded = 100 * 1e18;
        uint256 borrowAmountUSDPrice = 2000 * 1e18;
        uint256 ltv = 85e16;

        uint256 collateralNeeded =
            IlSolverMath.calculateCollateralAmount(borrowedAmountNeeded, borrowAmountUSDPrice, ltv);

        console.log("Large Amount Test:");
        console.log("  need to borrow:", borrowedAmountNeeded / 1e18, "tokens");
        console.log("  collateral needed:", collateralNeeded / 1e18, "USD");

        // Verify it works with the found collateral
        (, bool isEnough, uint256 totalBorrowed,) =
            IlSolverMath.hedgingLoop(collateralNeeded, borrowedAmountNeeded, borrowAmountUSDPrice, ltv);

        assertTrue(isEnough);
        assertGe(totalBorrowed, borrowedAmountNeeded);
    }

    // --- Precision and fractional iteration tests ---

    function test_fractionalIterations_exactMinimum() public {
        // Test that minimum collateral uses fractional iterations efficiently
        uint256 borrowedAmountNeeded = 1 * 1e18;
        uint256 borrowAmountUSDPrice = 2000 * 1e18;
        uint256 ltv = 90e16;

        uint256 minCollateral = IlSolverMath.calculateCollateralAmount(borrowedAmountNeeded, borrowAmountUSDPrice, ltv);

        (uint256 iterations, bool isEnough, uint256 totalBorrowed,) =
            IlSolverMath.hedgingLoop(minCollateral, borrowedAmountNeeded, borrowAmountUSDPrice, ltv);

        console.log("Exact Minimum Test:");
        console.log("  minCollateral:", minCollateral / 1e18);
        console.log("  iterations:", iterations / 1e16); // 2 decimals
        console.log("  totalBorrowed:", totalBorrowed / 1e18);

        assertTrue(isEnough);
        // With minimum, should use fractional iterations
        assertGt(iterations, 1e18); // More than 1.0
        assertLt(iterations, 8 * 1e18); // Less than max
        // Should borrow very close to exact amount needed
        assertApproxEqAbs(totalBorrowed, borrowedAmountNeeded, 1e15); // Within 0.001 tokens
    }

    function test_fractionalIterations_neverOverBorrow() public {
        // Verify we never borrow more than needed
        uint256 borrowedAmountNeeded = 5 * 1e18;
        uint256 borrowAmountUSDPrice = 1500 * 1e18;
        uint256 ltv = 80e16;

        uint256 collateral = IlSolverMath.calculateCollateralAmount(borrowedAmountNeeded, borrowAmountUSDPrice, ltv);

        (, bool isEnough, uint256 totalBorrowed,) =
            IlSolverMath.hedgingLoop(collateral, borrowedAmountNeeded, borrowAmountUSDPrice, ltv);

        assertTrue(isEnough);
        // Should borrow AT LEAST what's needed
        assertGe(totalBorrowed, borrowedAmountNeeded);
        // But not significantly more (allowing small rounding)
        assertLt(totalBorrowed, borrowedAmountNeeded + 1e15); // Less than 0.001 tokens extra
    }

    // --- Error case tests ---

    function test_revert_zeroPrice() public {
        vm.expectRevert(IlSolverMath.INVALID_INPUT.selector);
        wrapper.hedgingLoop(1000 * 1e18, 1 * 1e18, 0, 90e16);
    }

    function test_revert_zeroAmount() public {
        vm.expectRevert(IlSolverMath.INVALID_INPUT.selector);
        wrapper.hedgingLoop(1000 * 1e18, 0, 2000 * 1e18, 90e16);
    }

    function test_revert_zeroLTV() public {
        vm.expectRevert(IlSolverMath.INVALID_INPUT.selector);
        wrapper.hedgingLoop(1000 * 1e18, 1 * 1e18, 2000 * 1e18, 0);
    }

    function test_revert_ltvTooLow() public {
        // LTV below safety factor (2%)
        vm.expectRevert(IlSolverMath.INVALID_INPUT.selector);
        wrapper.hedgingLoop(1000 * 1e18, 1 * 1e18, 2000 * 1e18, 1e16); // 1%
    }

    function test_revert_insufficientCollateral() public {
        // Very low collateral, can't reach target even with max iterations
        uint256 collateralAmount = 100 * 1e18; // Only 100 USD
        uint256 borrowedAmount = 10 * 1e18; // Need 10 tokens
        uint256 borrowAmountUSD = 2000 * 1e18; // At $2000 each = $20,000 worth
        uint256 ltv = 90e16;

        vm.expectRevert(IlSolverMath.MAX_LOOP_ITERATIONS_REACHED.selector);
        wrapper.hedgingLoop(collateralAmount, borrowedAmount, borrowAmountUSD, ltv);
    }

    function test_revert_zeroCollateral() public {
        vm.expectRevert(IlSolverMath.INVALID_INPUT.selector);
        wrapper.hedgingLoop(0, 1 * 1e18, 2000 * 1e18, 90e16);
    }

    // --- Multiple LTV scenarios ---

    function test_differentLTVs_calculateMinimum() public {
        uint256 borrowedAmountNeeded = 1 * 1e18;
        uint256 borrowAmountUSDPrice = 2000 * 1e18;

        // Test various LTV values
        uint256[] memory ltvValues = new uint256[](4);
        ltvValues[0] = 70e16; // 70%
        ltvValues[1] = 80e16; // 80%
        ltvValues[2] = 90e16; // 90%
        ltvValues[3] = 95e16; // 95%

        console.log("LTV Comparison:");
        uint256 previousCollateral = type(uint256).max;

        for (uint256 i = 0; i < ltvValues.length; i++) {
            uint256 collateral =
                IlSolverMath.calculateCollateralAmount(borrowedAmountNeeded, borrowAmountUSDPrice, ltvValues[i]);

            console.log("  LTV %:", ltvValues[i] / 1e16);
            console.log("    collateral USD:", collateral / 1e18);

            // Higher LTV should need LESS collateral (more leverage)
            if (i > 0) {
                assertLt(collateral, previousCollateral);
            }
            previousCollateral = collateral;
        }
    }
}
