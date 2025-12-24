// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {IDelegationManager} from "../../src/interfaces/IDelegationManager.sol";
import {DelegatorSummary} from "../../src/models/Type.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/// @title DelegationManagerMock
/// @notice Mock implementation of IDelegationManager for testing purposes
contract DelegationManagerMock is IDelegationManager, Ownable, AccessControl {
    uint64 public totalDelegated;
    uint64 public totalUndelegated;
    uint64 public totalPendingWithdrawal;
    uint64 public nPendingWithdrawals;
    bytes32 public constant MANAGER_ROLE = keccak256("MANAGER_ROLE");
    uint256 public constant CORE_TO_EVM = 10 ** 10;

    constructor() Ownable(msg.sender) {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
    }

    // Track individual delegations for more detailed testing
    mapping(address => uint64) public delegatedAmount;

    /// @notice Delegates the sent value to validators
    function delegate(uint256 amount) external payable override {
        require(msg.value == amount, "Wrong delegated amount");
        uint64 delegateAmount = uint64(msg.value) / uint64(CORE_TO_EVM);
        totalDelegated += delegateAmount;
        delegatedAmount[msg.sender] += delegateAmount;
        emit Delegated(msg.sender, msg.value);
    }

    function setLiquidStakingManager(address _manager) external onlyOwner {
        _grantRole(MANAGER_ROLE, _manager);
    }

    /// @notice Undelegates the specified amount from validators
    /// @param amount The amount to undelegate
    function undelegate(uint256 amount) external override {
        uint64 undelegateAmount = uint64(amount) / uint64(CORE_TO_EVM);
        require(totalDelegated >= undelegateAmount, "Insufficient delegated amount");
        totalDelegated -= undelegateAmount;
        totalUndelegated += undelegateAmount;
        totalPendingWithdrawal += undelegateAmount;
        nPendingWithdrawals += 1;
        emit Undelegated(msg.sender, amount);
    }

    /// @notice Returns the delegation summary for this contract
    /// @return The delegator summary
    function delegationSummary() external view override returns (DelegatorSummary memory) {
        return DelegatorSummary({
            delegated: totalDelegated,
            undelegated: totalUndelegated,
            totalPendingWithdrawal: totalPendingWithdrawal,
            nPendingWithdrawals: nPendingWithdrawals,
            rewards: 0
        });
    }

    /// @notice Mock implementation of updateValidators
    /// @param _validators Array of validator addresses
    /// @param _weights Array of weights for each validator
    function updateValidators(address[] calldata _validators, uint64[] calldata _weights) external override {
        // Mock implementation - does nothing in tests
        // In a real scenario, this would redelegate tokens
    }

    // ============ Mock Helper Functions ============

    /// @notice Sets the delegation summary values for testing
    /// @param _delegated The delegated amount
    /// @param _undelegated The undelegated amount
    /// @param _totalPendingWithdrawal The total pending withdrawal amount
    /// @param _nPendingWithdrawals The number of pending withdrawals
    function setDelegationSummary(
        uint64 _delegated,
        uint64 _undelegated,
        uint64 _totalPendingWithdrawal,
        uint64 _nPendingWithdrawals
    ) external {
        totalDelegated = _delegated;
        totalUndelegated = _undelegated;
        totalPendingWithdrawal = _totalPendingWithdrawal;
        nPendingWithdrawals = _nPendingWithdrawals;
    }

    /// @notice Resets all mock state
    function reset() external {
        totalDelegated = 0;
        totalUndelegated = 0;
        totalPendingWithdrawal = 0;
        nPendingWithdrawals = 0;
    }

    /// @notice Simulates completing a pending withdrawal
    /// @param amount The amount to complete withdrawal for
    function completePendingWithdrawal(uint64 amount) external {
        require(totalPendingWithdrawal >= amount, "Insufficient pending withdrawal");
        require(nPendingWithdrawals > 0, "No pending withdrawals");
        totalPendingWithdrawal -= amount;
        nPendingWithdrawals -= 1;
    }

    /// @notice Transfer received unbonded/undelegated assets from validators to liquid staking manager
    function receiveBatch(uint256 batchAssets) external {
        require(hasRole(MANAGER_ROLE, msg.sender), "Caller is not a manager");

        // Transfer assets to recipient
        (bool success,) = payable(msg.sender).call{value: batchAssets}("");
        require(success, "transfer failed");
    }

    /// @notice Allows the mock to receive ETH
    receive() external payable {}
}
