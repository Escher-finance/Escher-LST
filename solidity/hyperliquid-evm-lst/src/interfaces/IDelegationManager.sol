// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {DelegatorSummary} from "../models/Type.sol";

/// @dev Interface of the IDelegationManager that handles delegation and undelegation
interface IDelegationManager {
    error EmptyValidatorSet();

    // Events
    event Delegated(address indexed sender, uint256 amount);
    event Undelegated(address indexed sender, uint64 amount);

    function setLiquidStakingManager(address _manager) external;

    /// @notice Delegates the sent value to validators
    function delegate() external payable;

    /// @notice Undelegates the specified amount from validators
    /// @param amount The amount to undelegate
    function undelegate(uint64 amount) external;

    /// @notice Returns the delegation summary for this contract
    /// @return The delegator summary from the precompile
    function delegationSummary() external view returns (DelegatorSummary memory);

    /// @notice Update validators and redelegate accordingly with new validator weight distribution
    /// @param _validators Array of validator addresses
    /// @param _weights Array of weights for each validator
    function updateValidators(address[] calldata _validators, uint64[] calldata _weights) external;

    /// @notice Receive Batch is function to transfer received unbonded/undelegated amount to liquid staking manager
    /// @param batchAssets Amount of unbonded assets that was received from validators
    function receiveBatch(uint256 batchAssets) external;
}
