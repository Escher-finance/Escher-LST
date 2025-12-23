// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import "@openzeppelin/contracts-upgradeable/access/Ownable2StepUpgradeable.sol";
import {UnbondBatch, UnbondRequest, Liquidity} from "../models/State.sol";
import {Rate} from "../models/Type.sol";

/// @dev Interface of the ILiquidStakingManager that handle liquid staking user interactions and operations.
interface ILiquidStakingManager {
    error TokenTransferFailure();

    /// @dev Emitted when user stakes some amount of native token
    event Bond(address indexed staker, uint256 value, address recipient);

    /// @dev Emitted when user request to unstake some amount of liquid staking token.
    /// @param user The address of the user who unstaked.
    /// @param shares The shares of the liquid staking token that the user wants to unstake.
    /// @param recipient The address of the user who will receive the asset back.
    /// @param requestId The unique ID of this unbond request.
    /// @param batchId The batch ID this request belongs to.
    event UnbondRequested(address indexed user, uint256 shares, address recipient, uint256 requestId, uint256 batchId);

    /// @dev Emitted when a new unbond batch is created.
    /// @param batchId The unique ID of the new batch.
    event BatchCreated(uint256 indexed batchId);

    /// @dev Emitted when a batch is submitted for undelegation.
    /// @param batchId The batch ID that was submitted.
    /// @param totalShares The total LST shares in the batch.
    /// @param totalAssets The total assets to be undelegated.
    /// @param nextActionTime The timestamp when the batch can be received.
    event BatchSubmitted(uint256 indexed batchId, uint256 totalShares, uint256 totalAssets, uint256 nextActionTime);

    /// @dev Emitted when a batch has received the undelegated tokens.
    /// @param batchId The batch ID that received tokens.
    /// @param totalAssets The total assets received.
    event BatchReceived(uint256 indexed batchId, uint256 totalAssets);

    /// @dev Emitted when a user claims their unbonded assets.
    /// @param user The address of the user who claimed.
    /// @param requestId The request ID that was claimed.
    /// @param assets The amount of assets claimed.
    /// @param recipient The address that received the assets.
    event UnbondClaimed(address indexed user, uint256 requestId, uint256 assets, address recipient);

    /// @notice Bond native token and get the share/liquid staking token to recipient according to the rate
    /// @param _assets amount of native token that is staked
    /// @param _recipient recipient address of the liquid staking token
    function bond(uint256 _assets, address _recipient) external payable;

    /// @notice Create unbond request of liquid staking token to receive native token back according to the rate
    /// @param _shares amount of shares/liquid staking token that will be unbonded
    /// @param _recipient recipient address of native token as unbonding result
    function unbondRequest(uint256 _shares, address _recipient) external returns (uint256);

    /// @notice Get delegation manager contract address
    /// @return address of delegation manager contract
    function getDelegationManager() external returns (address);

    /// @notice Submit the current pending batch for undelegation
    function submitBatch() external;

    /// @notice Mark a submitted batch as received after undelegation period
    /// @param batchId The ID of the batch to mark as received
    function receiveBatch(uint256 batchId) external;

    /// @notice Claim all unbonded assets for the caller
    function claimUnbond() external;

    /// @notice Claim unbonded assets for a specific request
    /// @param requestId The ID of the unbond request to claim
    function claimUnbondRequest(uint256 requestId) external;

    /// @notice Get the current pending batch ID (alias for compatibility)
    /// @return The current pending batch ID
    function getCurrentBatchId() external view returns (uint256);

    /// @notice Get batch information by ID
    /// @param batchId The batch ID to query
    /// @return The batch information
    function getBatch(uint256 batchId) external view returns (UnbondBatch memory);

    /// @notice Get unbond request information by ID
    /// @param requestId The request ID to query
    /// @return The unbond request information
    function getUnbondRequest(uint256 requestId) external view returns (UnbondRequest memory);

    /// @notice Get all request IDs for a user
    /// @param user The user address to query
    /// @return Array of request IDs
    function getUserRequestIds(address user) external view returns (uint256[] memory);

    function getLiquidity() external view returns (Liquidity memory);

    function rate() external view returns (Rate memory);
}
