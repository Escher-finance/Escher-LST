// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {ValidatorWeight} from "../models/Delegation.sol";

/// @dev Interface of the IValidatorManager that handle delegation and undelegation
interface IDelegationManager {
    //function set_validators(ValidatorWeight[] calldata _validators) external;

    function get_validators() external returns (ValidatorWeight[] memory);

    function delegate() external payable;

    function undelegate(uint256 amount) external;

    function totalRewards() external view returns (uint256);
}
