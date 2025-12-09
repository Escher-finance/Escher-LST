// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import {Test, console} from "forge-std/Test.sol";
import {IlSolverMath} from "../src/core/EscherMath.sol";
import {console} from "forge-std/console.sol";

contract IlSolverMathTest is Test {
    // --- sqrt tests ---

    function test_sqrt_basic() public {
        assertEq(IlSolverMath.sqrt(0), 0);
        assertEq(IlSolverMath.sqrt(1), 1);
        assertEq(IlSolverMath.sqrt(4), 2);
        assertEq(IlSolverMath.sqrt(16), 4);
        assertEq(IlSolverMath.sqrt(81), 9);
    }

    function test_sqrt_monotonic() public {
        uint256 a = 100;
        uint256 b = 10_000;
        uint256 sa = IlSolverMath.sqrt(a);
        uint256 sb = IlSolverMath.sqrt(b);
        assertLt(sa, sb);
    }

    // --- hedgingLoop tests ---

    function test_hedgingLoop_singleIterationEnough() public {
        // Simple case: one loop is enough to reach target
        uint256 collateralAmount = 2224 * 1e18;
        uint256 borrowedAmount = 1 * 1e18;
        uint256 borrowAmountUSD = 2000 * 1e18;
        uint256 ltv = 90e16; // 0.90 * 1e18

        uint256 iterations = IlSolverMath.hedgingLoop(collateralAmount, borrowedAmount, borrowAmountUSD, ltv);
        console.log("iterations", iterations);
        console.log("collateralAmount", collateralAmount);

        // Expect a single loop is sufficient
        assertEq(iterations, 2);
    }
    // need to do test on the LTV(safety factor), test on maximum loops, test on invalid input
}
