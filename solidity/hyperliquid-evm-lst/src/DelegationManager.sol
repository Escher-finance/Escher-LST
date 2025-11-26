// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;

import {ERC4626} from "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import {IERC20, ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {IDelegationManager} from "./interfaces/IDelegationManager.sol";
import {ValidatorWeight} from "./models/Delegation.sol";
import {CoreWriterLib, HLConstants, HLConversions} from "@hyper-evm-lib/src/CoreWriterLib.sol";

contract DelegationManager is IDelegationManager {
    ValidatorWeight[] validators;

    function add_validator(address validator, uint256 weight) external {
        validators.push(
            ValidatorWeight({validator: validator, weight: weight})
        );
    }

    function get_validators() external view returns (ValidatorWeight[] memory) {
        return validators;
    }

    function sendCore() external payable {
        uint64 hypeTokenIndex = HLConstants.hypeTokenIndex();
        CoreWriterLib.bridgeToCore(hypeTokenIndex, msg.value);
    }

    function depositStake(uint64 amount) external {
        CoreWriterLib.depositStake(amount);
    }

    function stake(uint64 amount) external payable {
        ValidatorWeight memory validator = validators[0];
        CoreWriterLib.delegateToken(validator.validator, amount, false);
    }

    function delegate() external payable {
        // Transfer HYPE tokens to core
        uint64 hypeTokenIndex = HLConstants.hypeTokenIndex();
        CoreWriterLib.bridgeToCore(hypeTokenIndex, msg.value);

        // Using data from the `TokenInfo` precompile, convert EVM amount to core decimals for staking operations
        uint64 coreAmount = HLConversions.evmToWei(hypeTokenIndex, msg.value);

        // Delegate the tokens to a validator
        ValidatorWeight memory validator = validators[0];
        CoreWriterLib.delegateToken(validator.validator, coreAmount, false);
    }

    function firstvalidator() external view returns (address) {
        return validators[0].validator;
    }

    function undelegate(uint256 amount) external {
        // Todo: implement it
    }

    function totalRewards() external pure returns (uint256) {
        return 1000;
    }
}
