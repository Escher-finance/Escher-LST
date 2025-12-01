// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "@openzeppelin/contracts-upgradeable/access/Ownable2StepUpgradeable.sol";

/// @dev Interface of the ILiquidStakingManager that handle liquid staking user interactions and operations.
interface ILiquidStakingManager {
    /// @dev Emitted when user stakes some amount of native token
    event Bond(address indexed staker, uint256 value, address recipient);

    /// @dev Emitted when user request to unstake some amount of liquid staking token.
    /// @param user The address of the user who unstaked.
    /// @param shares The shares of the liquid staking token that the user wants to unstake.
    /// @param recipient The address of the user who will receive the asset back.
    event UnbondRequest(
        address indexed user,
        uint256 shares,
        address recipient
    );

    function bond(uint256 _assets, address _recipient) external payable;

    function unbondRequest(uint256 _shares, address _recipient) external;

    function getDelegationManager() external returns (address);

    function setDelegationManager(address _delegate) external;
}
