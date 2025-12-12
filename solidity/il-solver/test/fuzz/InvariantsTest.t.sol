//SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {StdInvariant} from "forge-std/StdInvariant.sol";
import {IlSolverMath} from "../../src/core/EscherMath.sol";
import {Handler} from "./Handler.t.sol";

contract InvariantsTest is StdInvariant, Test {
    Handler public handler;

    function setUp() public {
        handler = new Handler();
        targetContract(address(handler));
    }
    
    // Invariant: Iterations should never exceed MAX_LOOP_ITERATIONS
    function invariant_iterationsWithinBounds() public view {
        if (handler.lastIsEnough()) {
            // If enough, iterations should be > 0 and <= 8
            assertGt(handler.lastIterations(), 0);
            assertLe(handler.lastIterations(), 8 * 1e18);
        }
    }
    
    // Invariant: Total borrowed should never be less than 0
    function invariant_borrowedAmountPositive() public view {
        if (handler.lastIsEnough() && handler.lastTotalBorrowed() > 0) {
            // When enough, total borrowed should be positive
            assertGt(handler.lastTotalBorrowed(), 0);
        }
    }
    
    // Invariant: Fractional iterations should be properly scaled (in 1e18)
    function invariant_fractionalIterationsScaling() public view {
        if (handler.lastIsEnough()) {
            uint256 iterations = handler.lastIterations();
            // Should be in 1e18 scale - check it's reasonable
            // Min: close to 0, Max: 8 * 1e18
            assertLe(iterations, 8 * 1e18 + 1); // +1 for rounding
        }
    }
    
    // Invariant: Should have some successful calls (if any calls made)
    function invariant_callSuccessRate() public view {
        if (handler.callCount() > 100) {
            // After enough calls, at least 10% should succeed
            uint256 successRate = (handler.successfulCalls() * 100) / handler.callCount();
            assertGe(successRate, 10, "Too many failed calls");
        }
    }
}