// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {DelegatorSummary} from "../models/Type.sol";

/// @dev Interface of the IDelegationManager that handles delegation and undelegation
interface IDelegationManager {
    error EmptyValidatorSet();
    error FailedDelegation();
    error FailedUndelegation();

    // Events
    event Delegated(address indexed sender, uint256 amount);
    event Undelegated(address indexed sender, uint256 amount);
    event ValidatorsUpdated(address[] validators, uint64[] weights);
    event BatchMoved(uint256 assets);
    event BatchReceived(uint256 assets);

    function setLiquidStakingManager(address _manager) external;

    /// @notice Delegates the sent value to validators
    function delegate(uint256 amount) external payable;

    /// @notice Undelegates the specified amount from validators
    /// @param amount The amount to undelegate
    function undelegate(uint256 amount) external;

    function active() external view returns (bool);

    /// @notice Returns the delegation summary for this contract
    /// @return The delegator summary from the precompile
    function delegationSummary() external view returns (DelegatorSummary memory);

    /// @notice Update validators and redelegate accordingly with new validator weight distribution
    /// @param _validators Array of validator addresses
    /// @param _weights Array of weights for each validator
    function updateValidators(address[] calldata _validators, uint64[] calldata _weights) external;

    /// @notice moveBatch is function to move batch assets from spot balance to Evm
    /// @param batchAssets Amount of unbonded assets that was received from validators
    function moveBatch(uint256 batchAssets) external;

    /// @notice Receive Batch is function to transfer received unbonded/undelegated amount to liquid staking manager
    /// @param batchAssets Amount of unbonded assets that was received from validators
    function receiveBatch(uint256 batchAssets) external;
}
