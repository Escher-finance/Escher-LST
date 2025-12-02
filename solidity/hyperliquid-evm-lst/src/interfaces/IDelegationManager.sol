// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {DelegatorSummary} from "../models/Type.sol";

/// @dev Interface of the IDelegationManager that handles delegation and undelegation
interface IDelegationManager {
    error EmptyValidatorSet();

    // Events
    event Delegated(address indexed sender, uint256 amount);
    event Undelegated(address indexed sender, uint64 amount);

    /// @notice Delegates the sent value to validators
    function delegate() external payable;

    /// @notice Undelegates the specified amount from validators
    /// @param amount The amount to undelegate
    function undelegate(uint64 amount) external;

    /// @notice Returns the delegation summary for this contract
    /// @return The delegator summary from the precompile
    function delegationSummary() external view returns (DelegatorSummary memory);
}
