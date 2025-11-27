// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {ValidatorWeight} from "../models/Delegation.sol";
import {PrecompileLib} from "@hyper-evm-lib/src/CoreWriterLib.sol";

/// @dev Interface of the IValidatorManager that handle delegation and undelegation
interface IDelegationManager {
    //function set_validators(ValidatorWeight[] calldata _validators) external;

    function get_validators() external returns (ValidatorWeight[] memory);

    function delegate() external payable;

    function undelegate(uint64 amount) external;

    function delegationSummary()
        external
        view
        returns (PrecompileLib.DelegatorSummary memory);
}
