// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Validator} from "../models/Type.sol";
import {IValidatorSetManager} from "../interfaces/IValidatorSetManager.sol";
import "@openzeppelin-upgradeable/contracts/proxy/utils/Initializable.sol";
import "@openzeppelin-upgradeable/contracts/access/Ownable2StepUpgradeable.sol";
import "@openzeppelin-upgradeable/contracts/proxy/utils/UUPSUpgradeable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin-upgradeable/contracts/access/AccessControlUpgradeable.sol";

/**
 * @title ValidatorSetManager
 * @notice validator set management for liquid staking
 */
contract ValidatorSetManager is
    IValidatorSetManager,
    Initializable,
    UUPSUpgradeable,
    AccessControlUpgradeable,
    Ownable2StepUpgradeable,
    ReentrancyGuard
{
    Validator[] private validators;
    mapping(address => uint256) private validatorIndex; // validator address => array index + 1
    uint64 public totalWeight;
    bytes32 public constant MANAGER_ROLE = keccak256("MANAGER_ROLE");

    // Required by UUPSUpgradeable - only owner can upgrade
    function _authorizeUpgrade(address newImplementation) internal override onlyOwner {}

    constructor() {
        _disableInitializers();
    }

    function initialize(address initialOwner) external initializer {
        // Checks that the initialOwner address is not zero.
        if (initialOwner == address(0)) revert InvalidAddress();
        __Ownable_init(initialOwner);
        __AccessControl_init();
        require(_grantRole(DEFAULT_ADMIN_ROLE, initialOwner), "failed to grant admin role");
    }

    function setDelegationManager(address manager) external onlyOwner {
        // Checks that the manager address is not zero.
        require(manager != address(0), "manager zero address");
        require(_grantRole(MANAGER_ROLE, manager), "failed to grant manager role");
    }

    /**
     * @notice Update validator set, requires minimum 1 validator and weight pair
     * @param newValidators Array of validator addresses
     * @param weights Array of corresponding weights
     */
    function updateValidators(address[] calldata newValidators, uint64[] calldata weights) external nonReentrant {
        require(hasRole(MANAGER_ROLE, msg.sender), "Caller is not a manager");
        uint256 newLength = newValidators.length;
        require(newLength > 0, "Requires minimum 1 validator");
        if (newLength != weights.length) revert ArrayLengthMismatch();

        uint256 oldLength = validators.length;
        // Clear existing mappings
        for (uint256 i = 0; i < oldLength;) {
            delete validatorIndex[validators[i].validator];
            unchecked {
                ++i;
            }
        }

        // Reset total weight
        totalWeight = 0;

        // Clear array
        delete validators;

        bool invalidAddress = false;
        bool invalidWeight = false;

        // Add new validators
        for (uint256 i = 0; i < newLength;) {
            address validatorAddress = newValidators[i];
            uint64 weight = weights[i];

            if (validatorAddress == address(0)) {
                invalidAddress = true;
                break;
            }

            if (weight == 0) {
                invalidWeight = true;
                break;
            }

            validators.push(Validator({validator: validatorAddress, weight: weight}));

            validatorIndex[validatorAddress] = validators.length;
            totalWeight += weight;

            unchecked {
                ++i;
            }
        }

        if (invalidAddress) revert InvalidAddress();
        if (invalidWeight) revert InvalidWeight();

        emit ValidatorSetBatchUpdated(newLength);
    }

    /**
     * @notice Get validator details by address
     * @param validatorAddr Address of the validator
     * @return validatorAddress The validator address
     * @return weight The validator weight
     */
    function getValidator(address validatorAddr) external view returns (address validatorAddress, uint64 weight) {
        uint256 index = validatorIndex[validatorAddr];
        require(index != 0, "Validator not found");

        index--;
        Validator memory v = validators[index];
        return (v.validator, v.weight);
    }

    /**
     * @notice Get all validators
     * @return Array of all validators
     */
    function getAllValidators() external view returns (Validator[] memory) {
        return validators;
    }

    /**
     * @notice Get validator count
     * @return Number of validators in the set
     */
    function getValidatorCount() external view returns (uint256) {
        return validators.length;
    }

    /**
     * @notice Get total weight of all validators
     * @return Total weight of validators
     */
    function getTotalWeight() external view returns (uint64) {
        return totalWeight;
    }

    /**
     * @notice Check if a validator exists
     * @param validatorAddr Address to check
     * @return bool True if validator exists
     */
    function validatorExists(address validatorAddr) external view returns (bool) {
        return validatorIndex[validatorAddr] != 0;
    }
}
