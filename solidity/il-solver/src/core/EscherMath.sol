// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";
import {console} from "forge-std/console.sol";
library IlSolverMath {
    error MAX_LOOP_ITERATIONS_REACHED();
    error INVALID_INPUT();
    uint256 public constant MAX_LOOP_ITERATIONS = 8;
    uint256 public constant LTV_SAFTY_FACTOR = 2e16;

    function sqrt(uint256 x) internal pure returns (uint256) {
        return Math.sqrt(x);
    }
    function log10(uint256 x) internal pure returns (uint256) {
        return Math.log10(x);
    }
    /**
     * @dev This function is used to calculate the number of iterations needed to reach the borrowed amount needed.
     * @param collateralAmount The amount of collateral to be used in token decimals (1e18).
     * @param borrowedAmountNeeded The amount of borrowed tokens needed token decimals (1e18).
     * @param borrowAmountUSDPrice The price of the borrowed tokens in USD token decimals (1e18).
     * @param ltv The LTV of the collateral in 1e16.
     * @return iterations The number of iterations needed to reach the borrowed amount needed.
     *  After n loops starting with L_0 collateral:
    H_n = (L_0 * LTV / P_0) * (1 - LTV^n) / (1 - LTV)
     */
    function hedgingLoop(uint256 collateralAmount, uint256 borrowedAmountNeeded, uint256 borrowAmountUSDPrice, uint256 ltv) internal pure returns (uint256 iterations) {
        if (borrowAmountUSDPrice == 0 || borrowedAmountNeeded == 0 || ltv == 0 || collateralAmount == 0) {
            revert INVALID_INPUT();
        }
        // Reduce target LTV by the safety factor to avoid exceeding the limit.
        if (ltv <= LTV_SAFTY_FACTOR) revert INVALID_INPUT();
        uint256 ltvMax = ltv - LTV_SAFTY_FACTOR;

        // Track total collateral and borrow amounts in USD (1e18 scale) and tokens.
        uint256 collateralUSD = collateralAmount;
        uint256 totalBorrowedUSD = 0;
        uint256 totalBorrowedToken = 0;

        for (uint256 i = 0; i < MAX_LOOP_ITERATIONS; ++i) {
            // Maximum borrowable USD at this collateral level.
            uint256 maxBorrowableUSD = Math.mulDiv(collateralUSD, ltvMax, 1e18);
            if (maxBorrowableUSD <= totalBorrowedUSD) {
                break; // already at (or above) allowed LTV
            }

            uint256 remainingCapacityUSD = maxBorrowableUSD - totalBorrowedUSD;
            // Convert USD capacity to borrowed token amount using the price.
            uint256 borrowThisLoopToken = Math.mulDiv(remainingCapacityUSD, 1e18, borrowAmountUSDPrice);

            totalBorrowedUSD += remainingCapacityUSD;
            totalBorrowedToken += borrowThisLoopToken;
            collateralUSD += remainingCapacityUSD; // borrowed funds are re-deposited as collateral

            iterations = i + 1;
            if (totalBorrowedToken >= borrowedAmountNeeded) {
                return iterations;
            }
        }

        if (iterations == MAX_LOOP_ITERATIONS) revert MAX_LOOP_ITERATIONS_REACHED();
        return iterations;
    }
}
