// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

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

    function initialize(address initialOwner) public initializer {
        // Checks that the initialOwner address is not zero.
        if (initialOwner == address(0)) revert InvalidAddress();
        __Ownable_init(initialOwner);
        __AccessControl_init();
        _grantRole(DEFAULT_ADMIN_ROLE, initialOwner);
    }

    function setDelegationManager(address _manager) external onlyOwner {
        _grantRole(MANAGER_ROLE, _manager);
    }

    /**
     * @notice Update validator set, requires minimum 1 validator and weight pair
     * @param _validators Array of validator addresses
     * @param _weights Array of corresponding weights
     */
    function updateValidators(address[] calldata _validators, uint64[] calldata _weights) external nonReentrant {
        require(hasRole(MANAGER_ROLE, msg.sender), "Caller is not a manager");
        uint256 length = _validators.length;
        require(length > 0, "Requires minimum 1 validator");
        if (length != _weights.length) revert ArrayLengthMismatch();

        // Clear existing mappings
        for (uint256 i = 0; i < validators.length;) {
            delete validatorIndex[validators[i].validator];
            unchecked {
                ++i;
            }
        }

        // Reset total weight
        totalWeight = 0;

        // Clear array
        delete validators;

        // Add new validators
        for (uint256 i = 0; i < length;) {
            address validatorAddress = _validators[i];
            uint64 weight = _weights[i];

            if (validatorAddress == address(0)) revert InvalidAddress();
            if (weight == 0) revert InvalidWeight();

            validators.push(Validator({validator: validatorAddress, weight: weight}));

            validatorIndex[validatorAddress] = validators.length;
            totalWeight += weight;

            unchecked {
                ++i;
            }
        }

        emit ValidatorSetBatchUpdated(length);
    }

    /**
     * @notice Get validator details by address
     * @param _validator Address of the validator
     * @return validatorAddress The validator address
     * @return weight The validator weight
     */
    function getValidator(address _validator) external view returns (address validatorAddress, uint64 weight) {
        uint256 index = validatorIndex[_validator];
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
     * @param _validator Address to check
     * @return bool True if validator exists
     */
    function validatorExists(address _validator) external view returns (bool) {
        return validatorIndex[_validator] != 0;
    }
}
