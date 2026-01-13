// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";

library IlSolverMath {
    error MAX_LOOP_ITERATIONS_REACHED();
    error INVALID_INPUT();
    uint256 public constant MAX_LOOP_ITERATIONS = 8;
    uint256 public constant LTV_SAFTY_FACTOR = 2e16;
    // Epsilon tolerance for token amount comparison to account for rounding errors
    // 0.000001 tokens (in 18 decimals) = 1e12
    uint256 public constant TOKEN_AMOUNT_EPSILON = 0.000001 ether;

    function _validInput(
        uint256 collateralAmount,
        uint256 borrowedAmountNeeded,
        uint256 borrowedTokenUsdPrice,
        uint8 borrowedTokenDecimals,
        uint8 collateralTokenDecimals,
        uint256 ltv
    ) internal pure {
        if (
            borrowedTokenUsdPrice == 0 || borrowedAmountNeeded == 0 || ltv == 0 || collateralAmount == 0
                || ltv <= LTV_SAFTY_FACTOR
        ) {
            revert INVALID_INPUT();
        }
        if (
            borrowedTokenDecimals == 0 || borrowedTokenDecimals > 18 || collateralTokenDecimals == 0
                || collateralTokenDecimals > 18
        ) {
            revert INVALID_INPUT();
        }
    }

    modifier validInput(
        uint256 collateralAmount,
        uint256 borrowedAmountNeeded,
        uint256 borrowedTokenUsdPrice,
        uint8 borrowedTokenDecimals,
        uint8 collateralTokenDecimals,
        uint256 ltv
    ) {
        _validInput(
            collateralAmount,
            borrowedAmountNeeded,
            borrowedTokenUsdPrice,
            borrowedTokenDecimals,
            collateralTokenDecimals,
            ltv
        );
        _;
    }

    /**
     * @dev This function is used to calculate the number of iterations needed to reach the borrowed amount needed.
     * @param collateralAmount The amount of collateral to be used; price must equal 1 USD.
     * @param borrowedAmountNeeded The amount of borrowed tokens needed token (18 decimals).
     * @param borrowedTokenUsdPrice The price of the borrowed token in USD (18 decimals).
     * @param borrowedTokenDecimals The number of decimals of the borrowed token; must be between 1 and 18.
     * @param collateralTokenDecimals The number of decimals of the collateral token; must be between 1 and 18.
     * @param ltv The LTV of the collateral in 1e16.
     * @return iterations The number of iterations needed to reach the borrowed amount needed.
     *  After n loops starting with L_0 collateral:
     * LTV / P_0) * (1 - LTV^n) / (1 - LTV)
     *
     */
    function hedgingLoop(
        uint256 collateralAmount,
        uint256 borrowedAmountNeeded,
        uint256 borrowedTokenUsdPrice,
        uint8 borrowedTokenDecimals,
        uint8 collateralTokenDecimals,
        uint256 ltv
    )
        internal
        pure
        validInput(
            collateralAmount,
            borrowedAmountNeeded,
            borrowedTokenUsdPrice,
            borrowedTokenDecimals,
            collateralTokenDecimals,
            ltv
        )
        returns (uint256 iterations, bool isEnough, uint256 totalBorrowedToken, uint256 ltvUsed)
    {
        isEnough = false;
        uint256 ltvMax = ltv - LTV_SAFTY_FACTOR;
        uint256 borrowedTokenScale = 10 ** (18 - borrowedTokenDecimals);
        uint256 collateralTokenScale = 10 ** (18 - collateralTokenDecimals);

        // Normalized amount to 18 decimals
        uint256 borrowedAmountNeededNorm = borrowedAmountNeeded * borrowedTokenScale;

        // Calculate target USD value once to avoid rounding errors in comparison
        uint256 targetBorrowUsd = Math.mulDiv(borrowedAmountNeededNorm, borrowedTokenUsdPrice, 1e18);

        // Track total collateral and borrow amounts in Usd (1e18 scale) and tokens.
        uint256 collateralUsd = collateralAmount * collateralTokenScale;
        uint256 totalBorrowedUsd = 0;
        uint256 totalBorrowedTokenNorm = 0;

        for (uint256 i = 0; i < MAX_LOOP_ITERATIONS; ++i) {
            // Maximum borrowable Usd at this collateral level.
            uint256 maxBorrowableUsd = Math.mulDiv(collateralUsd, ltvMax, 1e18);
            if (maxBorrowableUsd <= totalBorrowedUsd) {
                break; // already at (or above) allowed LTV
            }

            uint256 remainingCapacityUsd = maxBorrowableUsd - totalBorrowedUsd;

            // Calculate how much more we need to borrow (in USD)
            uint256 usdStillNeeded = targetBorrowUsd - totalBorrowedUsd;

            // Borrow the MINIMUM of: what we need vs what we can borrow
            // This simulates a "partial/fractional" iteration
            uint256 borrowThisLoopUsd = Math.min(remainingCapacityUsd, usdStillNeeded);
            uint256 borrowThisLoopToken = Math.mulDiv(borrowThisLoopUsd, 1e18, borrowedTokenUsdPrice);

            totalBorrowedUsd += borrowThisLoopUsd;
            totalBorrowedTokenNorm += borrowThisLoopToken;
            collateralUsd += borrowThisLoopUsd; // borrowed funds are re-deposited as collateral

            // Check if we've reached the target using token amount with epsilon tolerance
            // This accounts for rounding errors from USD->token conversions
            // We check if totalBorrowedTokenNorm >= borrowedAmountNeededNorm - epsilon
            // This means we're within epsilon of the target (or above it)
            if (totalBorrowedTokenNorm + TOKEN_AMOUNT_EPSILON >= borrowedAmountNeededNorm) {
                isEnough = true;
                // Calculate fractional iterations (in 1e18 scale)
                // iterations = completed full iterations + (partial amount borrowed / max could borrow)
                uint256 fractionOfIteration = Math.mulDiv(borrowThisLoopUsd, 1e18, remainingCapacityUsd);
                iterations = (i * 1e18) + fractionOfIteration;
                totalBorrowedToken = totalBorrowedTokenNorm / borrowedTokenScale;
                return (iterations, isEnough, totalBorrowedToken, ltvUsed);
            }

            // This was a full iteration, continue
            iterations = (i + 1) * 1e18;
        }

        if (iterations == MAX_LOOP_ITERATIONS * 1e18) revert MAX_LOOP_ITERATIONS_REACHED();
        totalBorrowedToken = totalBorrowedTokenNorm / borrowedTokenScale;
        return (iterations, isEnough, totalBorrowedToken, 0);
    }

    /**
     * @dev This function uses binary search to find the minimum collateral needed to reach the borrowed amount needed.
     * @param borrowedAmountNeeded The amount of borrowed tokens needed token decimals (1e18).
     * @param borrowedTokenUsdPrice The price of the borrowed token in USD token decimals (1e18).
     * @param ltv The LTV of the collateral in 1e16.
     * @return collateralAmountNeeded The minimum amount of collateral needed to reach the borrowed amount needed.
     *
     */
    function calculateCollateralAmount(uint256 borrowedAmountNeeded, uint256 borrowedTokenUsdPrice, uint256 ltv)
        internal
        pure
        returns (uint256 collateralAmountNeeded)
    {
        // Manual input validation (can't use modifier since collateral is unknown)
        if (borrowedTokenUsdPrice == 0 || borrowedAmountNeeded == 0 || ltv == 0) {
            revert INVALID_INPUT();
        }
        if (ltv <= LTV_SAFTY_FACTOR) revert INVALID_INPUT();

        uint256 ltvMax = ltv - LTV_SAFTY_FACTOR;

        // Binary search bounds
        uint256 borrowedUsd = Math.mulDiv(borrowedAmountNeeded, borrowedTokenUsdPrice, 1e18);

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
            (, bool isEnough,,) = hedgingLoop(mid, borrowedAmountNeeded, borrowedTokenUsdPrice, ltv);

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
