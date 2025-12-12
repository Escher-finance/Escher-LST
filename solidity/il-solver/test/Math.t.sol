// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import {Test, console} from "forge-std/Test.sol";
import {IlSolverMath} from "../src/core/EscherMath.sol";
import {console} from "forge-std/console.sol";

contract IlSolverMathTest is Test {
    // --- sqrt tests ---


    // --- hedgingLoop tests ---

    function test_hedgingLoop_singleIterationEnough() public {
        // Simple case: one loop is enough to reach target
        uint256 collateralAmount = 2224 * 1e18;
        uint256 borrowedAmount = 1 * 1e18;
        uint256 borrowAmountUSD = 2000 * 1e18;
        uint256 ltv = 90e16; // 0.90 * 1e18

        (uint256 iterations,bool isEnough,uint256 totalBorrowedToken,uint256 ltvUsed) = IlSolverMath.hedgingLoop(collateralAmount, borrowedAmount, borrowAmountUSD, ltv);
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

        uint256 collateralAmountNeeded = IlSolverMath.calculateCollateralAmount(borrowedAmountNeeded, borrowAmountUSDPrice, ltv);
        console.log("collateralAmountNeeded", collateralAmountNeeded);
        assertEq(collateralAmountNeeded, 1136363636363636363636);
    }
}
