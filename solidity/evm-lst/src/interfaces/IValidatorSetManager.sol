pragma solidity 0.8.28;

import {Validator} from "../models/Type.sol";

interface IValidatorSetManager {
    // Errors
    error InvalidWeight();
    error InvalidAddress();
    error ArrayLengthMismatch();

    // Events

    /// @dev Emitted when validator set is updated.
    /// @param validatorCount The total count of validators that are updated
    event ValidatorSetBatchUpdated(uint256 validatorCount);

    function setDelegationManager(address _manager) external;

    function updateValidators(address[] calldata _validators, uint64[] calldata _weights) external;

    function getValidator(address _validator) external view returns (address validatorAddress, uint64 weight);

    function getAllValidators() external view returns (Validator[] memory);

    function getValidatorCount() external view returns (uint256);

    function validatorExists(address _validator) external view returns (bool);

    function getTotalWeight() external view returns (uint64);
}
