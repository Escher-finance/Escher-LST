// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;

import {ERC4626} from "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import {IERC20, ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {IDelegationManager} from "./interfaces/IDelegationManager.sol";
import {ValidatorWeight} from "./models/Delegation.sol";
import {CoreWriterLib, HLConstants, HLConversions, PrecompileLib} from "@hyper-evm-lib/src/CoreWriterLib.sol";

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

    function delegate() external payable {
        uint256 evmAmount = msg.value;
        // Transfer HYPE tokens to core
        uint64 hypeTokenIndex = HLConstants.hypeTokenIndex();
        CoreWriterLib.bridgeToCore(hypeTokenIndex, evmAmount);

        // Using data from the `TokenInfo` precompile, convert EVM amount to core decimals for staking operations
        uint64 coreAmount = HLConversions.evmToWei(hypeTokenIndex, evmAmount);
        // transfer from core to staking balance
        CoreWriterLib.depositStake(coreAmount);

        ValidatorWeight memory validator = validators[0];
        CoreWriterLib.delegateToken(validator.validator, coreAmount, false);
    }

    function firstvalidator() external view returns (address) {
        return validators[0].validator;
    }

    function undelegate(uint64 coreAmount) external {
        // Undelegate tokens from the validator
        CoreWriterLib.delegateToken(validators[0].validator, coreAmount, true);

        // Withdraw the tokens from staking
        CoreWriterLib.withdrawStake(coreAmount);
    }

    function delegationSummary()
        external
        view
        returns (PrecompileLib.DelegatorSummary memory)
    {
        return PrecompileLib.delegatorSummary(address(this));
    }
}
