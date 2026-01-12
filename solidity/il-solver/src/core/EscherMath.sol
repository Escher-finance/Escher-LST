// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";

library IlSolverMath {
    error MAX_LOOP_ITERATIONS_REACHED();
    error INVALID_INPUT();
    uint256 public constant MAX_LOOP_ITERATIONS = 8;
    uint256 public constant LTV_SAFTY_FACTOR = 2e16;

    function _validInput(
        uint256 collateralAmount,
        uint256 borrowedAmountNeeded,
        uint256 borrowAmountUsdPrice,
        uint256 ltv
    ) internal pure {
        if (
            borrowAmountUsdPrice == 0 || borrowedAmountNeeded == 0 || ltv == 0 || collateralAmount == 0
                || ltv <= LTV_SAFTY_FACTOR
        ) {
            revert INVALID_INPUT();
        }
    }

    modifier validInput(
        uint256 collateralAmount,
        uint256 borrowedAmountNeeded,
        uint256 borrowAmountUsdPrice,
        uint256 ltv
    ) {
        _validInput(collateralAmount, borrowedAmountNeeded, borrowAmountUsdPrice, ltv);
        _;
    }

    /**
     * @dev This function is used to calculate the number of iterations needed to reach the borrowed amount needed.
     * @param collateralAmount The amount of collateral to be used in token decimals (1e18).
     * @param borrowedAmountNeeded The amount of borrowed tokens needed token decimals (1e18).
     * @param borrowAmountUsdPrice The price of the borrowed tokens in Usd token decimals (1e18).
     * @param ltv The LTV of the collateral in 1e16.
     * @return iterations The number of iterations needed to reach the borrowed amount needed.
     *  After n loops starting with L_0 collateral:
     * LTV / P_0) * (1 - LTV^n) / (1 - LTV)
     *
     */
    function hedgingLoop(
        uint256 collateralAmount,
        uint256 borrowedAmountNeeded,
        uint256 borrowAmountUsdPrice,
        uint256 ltv
    )
        internal
        pure
        validInput(collateralAmount, borrowedAmountNeeded, borrowAmountUsdPrice, ltv)
        returns (uint256 iterations, bool isEnough, uint256 totalBorrowedToken, uint256 ltvUsed)
    {
        isEnough = false;
        uint256 ltvMax = ltv - LTV_SAFTY_FACTOR;

        // Track total collateral and borrow amounts in Usd (1e18 scale) and tokens.
        uint256 collateralUsd = collateralAmount;
        uint256 totalBorrowedUsd = 0;
        totalBorrowedToken = 0;

        for (uint256 i = 0; i < MAX_LOOP_ITERATIONS; ++i) {
            // Maximum borrowable Usd at this collateral level.
            uint256 maxBorrowableUsd = Math.mulDiv(collateralUsd, ltvMax, 1e18);
            if (maxBorrowableUsd <= totalBorrowedUsd) {
                break; // already at (or above) allowed LTV
            }

            uint256 remainingCapacityUsd = maxBorrowableUsd - totalBorrowedUsd;

            // Calculate how much more we need to borrow (in tokens)
            uint256 tokensStillNeeded = borrowedAmountNeeded - totalBorrowedToken;
            uint256 usdStillNeeded = Math.mulDiv(tokensStillNeeded, borrowAmountUsdPrice, 1e18);

            // Borrow the MINIMUM of: what we need vs what we can borrow
            // This simulates a "partial/fractional" iteration
            uint256 borrowThisLoopUsd = Math.min(remainingCapacityUsd, usdStillNeeded);
            uint256 borrowThisLoopToken = Math.mulDiv(borrowThisLoopUsd, 1e18, borrowAmountUsdPrice);

            totalBorrowedUsd += borrowThisLoopUsd;
            totalBorrowedToken += borrowThisLoopToken;
            collateralUsd += borrowThisLoopUsd; // borrowed funds are re-deposited as collateral

            uint256 targetBorrowUsd = Math.mulDiv(borrowedAmountNeeded, borrowAmountUsdPrice, 1e18);
            if (totalBorrowedUsd >= targetBorrowUsd) {
                isEnough = true;
                // Calculate fractional iterations (in 1e18 scale)
                // iterations = completed full iterations + (partial amount borrowed / max could borrow)
                uint256 fractionOfIteration = Math.mulDiv(borrowThisLoopUsd, 1e18, remainingCapacityUsd);
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
     * @param borrowAmountUsdPrice The price of the borrowed tokens in Usd token decimals (1e18).
     * @param ltv The LTV of the collateral in 1e16.
     * @return collateralAmountNeeded The minimum amount of collateral needed to reach the borrowed amount needed.
     *
     */
    function calculateCollateralAmount(uint256 borrowedAmountNeeded, uint256 borrowAmountUsdPrice, uint256 ltv)
        internal
        pure
        returns (uint256 collateralAmountNeeded)
    {
        // Manual input validation (can't use modifier since collateral is unknown)
        if (borrowAmountUsdPrice == 0 || borrowedAmountNeeded == 0 || ltv == 0) {
            revert INVALID_INPUT();
        }
        if (ltv <= LTV_SAFTY_FACTOR) revert INVALID_INPUT();

        uint256 ltvMax = ltv - LTV_SAFTY_FACTOR;

        // Binary search bounds
        uint256 borrowedUsd = Math.mulDiv(borrowedAmountNeeded, borrowAmountUsdPrice, 1e18);

        // Lower bound: We're limited to MAX_LOOP_ITERATIONS, so can't reach theoretical infinite minimum
        // Use a more realistic lower bound: ~half of what we'd need with just one iteration
        // This ensures we don't start too low and hit MAX_LOOP_ITERATIONS error
        uint256 high = Math.mulDiv(borrowedUsd, 1e18, ltvMax); // Single iteration need
        uint256 low = high / 2; // Conservative lower bound

        // The binary search will find the actual minimum between these bounds

        uint256 result = high; // Default to high if we don't find better

        // Binary search for minimum collateral
        while (low <= high) {
            uint256 mid = (low + high) / 2;

            // Test if this collateral amount is sufficient
            (, bool isEnough,,) = hedgingLoop(mid, borrowedAmountNeeded, borrowAmountUsdPrice, ltv);

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
