// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";
import {console} from "forge-std/console.sol";

library IlSolverMath {
    error MAX_LOOP_ITERATIONS_REACHED();
    error INVALID_INPUT();
    uint256 public constant MAX_LOOP_ITERATIONS = 8;
    uint256 public constant LTV_SAFTY_FACTOR = 2e16;

    modifier validInput(
        uint256 collateralAmount,
        uint256 borrowedAmountNeeded,
        uint256 borrowAmountUSDPrice,
        uint256 ltv
    ) {
        if (borrowAmountUSDPrice == 0 || borrowedAmountNeeded == 0 || ltv == 0 || collateralAmount == 0) {
            revert INVALID_INPUT();
        }
        if (ltv <= LTV_SAFTY_FACTOR) revert INVALID_INPUT();
        _;
    }

    /**
     * @dev This function is used to calculate the number of iterations needed to reach the borrowed amount needed.
     * @param collateralAmount The amount of collateral to be used in token decimals (1e18).
     * @param borrowedAmountNeeded The amount of borrowed tokens needed token decimals (1e18).
     * @param borrowAmountUSDPrice The price of the borrowed tokens in USD token decimals (1e18).
     * @param ltv The LTV of the collateral in 1e16.
     * @return iterations The number of iterations needed to reach the borrowed amount needed.
     *  After n loops starting with L_0 collateral:
     * LTV / P_0) * (1 - LTV^n) / (1 - LTV)
     *
     */
    function hedgingLoop(
        uint256 collateralAmount,
        uint256 borrowedAmountNeeded,
        uint256 borrowAmountUSDPrice,
        uint256 ltv
    )
        internal
        pure
        validInput(collateralAmount, borrowedAmountNeeded, borrowAmountUSDPrice, ltv)
        returns (uint256 iterations, bool isEnough, uint256 totalBorrowedToken, uint256 ltvUsed)
    {
        isEnough = false;
        uint256 ltvMax = ltv - LTV_SAFTY_FACTOR;

        // Track total collateral and borrow amounts in USD (1e18 scale) and tokens.
        uint256 collateralUSD = collateralAmount;
        uint256 totalBorrowedUSD = 0;
        totalBorrowedToken = 0;

        for (uint256 i = 0; i < MAX_LOOP_ITERATIONS; ++i) {
            // Maximum borrowable USD at this collateral level.
            uint256 maxBorrowableUSD = Math.mulDiv(collateralUSD, ltvMax, 1e18);
            if (maxBorrowableUSD <= totalBorrowedUSD) {
                break; // already at (or above) allowed LTV
            }

            uint256 remainingCapacityUSD = maxBorrowableUSD - totalBorrowedUSD;

            // Calculate how much more we need to borrow (in tokens)
            uint256 tokensStillNeeded = borrowedAmountNeeded - totalBorrowedToken;
            uint256 usdStillNeeded = Math.mulDiv(tokensStillNeeded, borrowAmountUSDPrice, 1e18);

            // Borrow the MINIMUM of: what we need vs what we can borrow
            // This simulates a "partial/fractional" iteration
            uint256 borrowThisLoopUSD = Math.min(remainingCapacityUSD, usdStillNeeded);
            uint256 borrowThisLoopToken = Math.mulDiv(borrowThisLoopUSD, 1e18, borrowAmountUSDPrice);

            totalBorrowedUSD += borrowThisLoopUSD;
            totalBorrowedToken += borrowThisLoopToken;
            collateralUSD += borrowThisLoopUSD; // borrowed funds are re-deposited as collateral

            if (totalBorrowedToken >= borrowedAmountNeeded) {
                isEnough = true;
                // Calculate fractional iterations (in 1e18 scale)
                // iterations = completed full iterations + (partial amount borrowed / max could borrow)
                uint256 fractionOfIteration = Math.mulDiv(borrowThisLoopUSD, 1e18, remainingCapacityUSD);
                iterations = (i * 1e18) + fractionOfIteration;
                return (iterations, isEnough, totalBorrowedToken, ltvUsed);
            }

            // This was a full iteration, continue
            iterations = (i + 1) * 1e18;
        }

        if (iterations == MAX_LOOP_ITERATIONS * 1e18) revert MAX_LOOP_ITERATIONS_REACHED();
        return (iterations, isEnough, totalBorrowedToken, 0);
    }

    /**
     * @dev This function uses binary search to find the minimum collateral needed to reach the borrowed amount needed.
     * @param borrowedAmountNeeded The amount of borrowed tokens needed token decimals (1e18).
     * @param borrowAmountUSDPrice The price of the borrowed tokens in USD token decimals (1e18).
     * @param ltv The LTV of the collateral in 1e16.
     * @return collateralAmountNeeded The minimum amount of collateral needed to reach the borrowed amount needed.
     *
     */
    function calculateCollateralAmount(uint256 borrowedAmountNeeded, uint256 borrowAmountUSDPrice, uint256 ltv)
        internal
        pure
        returns (uint256 collateralAmountNeeded)
    {
        // Manual input validation (can't use modifier since collateral is unknown)
        if (borrowAmountUSDPrice == 0 || borrowedAmountNeeded == 0 || ltv == 0) {
            revert INVALID_INPUT();
        }
        if (ltv <= LTV_SAFTY_FACTOR) revert INVALID_INPUT();

        uint256 ltvMax = ltv - LTV_SAFTY_FACTOR;

        // Binary search bounds
        uint256 borrowedUSD = Math.mulDiv(borrowedAmountNeeded, borrowAmountUSDPrice, 1e18);

        // Lower bound: We're limited to MAX_LOOP_ITERATIONS, so can't reach theoretical infinite minimum
        // Use a more realistic lower bound: ~half of what we'd need with just one iteration
        // This ensures we don't start too low and hit MAX_LOOP_ITERATIONS error
        uint256 high = Math.mulDiv(borrowedUSD, 1e18, ltvMax); // Single iteration need
        uint256 low = high / 2; // Conservative lower bound

        // The binary search will find the actual minimum between these bounds

        uint256 result = high; // Default to high if we don't find better

        // Binary search for minimum collateral
        while (low <= high) {
            uint256 mid = (low + high) / 2;

            // Test if this collateral amount is sufficient
            (, bool isEnough,,) = hedgingLoop(mid, borrowedAmountNeeded, borrowAmountUSDPrice, ltv);

            if (isEnough) {
                result = mid;
                high = mid - 1;
            } else {
                // Need more collateral
                low = mid + 1;
            }
        }

        return result;
    }
}
