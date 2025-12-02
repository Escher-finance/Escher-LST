// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {ERC4626} from "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import {IERC20, ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {IDelegationManager} from "./interfaces/IDelegationManager.sol";
import {IValidatorSetManager} from "./interfaces/IValidatorSetManager.sol";
import {Validator, DelegatorSummary} from "./models/Type.sol";
import {CoreWriterLib, HLConstants, HLConversions, PrecompileLib} from "@hyper-evm-lib/src/CoreWriterLib.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts-upgradeable/access/Ownable2StepUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

contract DelegationManager is
    IDelegationManager,
    Initializable,
    UUPSUpgradeable,
    Ownable2StepUpgradeable,
    PausableUpgradeable,
    ReentrancyGuard
{
    IValidatorSetManager validatorManager;

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    function initialize(
        address owner,
        address _validatorManager
    ) public initializer {
        // Checks that the initialOwner address is not zero.
        require(owner != address(0), "zero address");
        __Ownable_init(owner);
        __Pausable_init();

        validatorManager = IValidatorSetManager(_validatorManager);
    }

    function _authorizeUpgrade(address) internal override onlyOwner {}

    /**
     * @notice Calculate stake distribution for a given amount
     * @param _amount Total amount to distribute
     * @return addresses Array of validator addresses
     * @return amounts Array of amounts to stake to each validator
     */
    function calculateStakeDistribution(
        uint64 _amount,
        Validator[] memory validators
    )
        internal
        view
        returns (address[] memory addresses, uint64[] memory amounts)
    {
        uint64 totalWeight = validatorManager.getTotalWeight();
        uint256 length = validators.length;
        if (length == 0) revert EmptyValidatorSet();

        addresses = new address[](length);
        amounts = new uint64[](length);

        uint64 distributed = 0;

        for (uint64 i = 0; i < length; ) {
            Validator memory v = validators[i];

            addresses[i] = v.validator;

            // Last validator gets remaining amount to handle rounding
            if (i == length - 1) {
                amounts[i] = _amount - distributed;
            } else {
                amounts[i] = (_amount * v.weight) / totalWeight;
                distributed += amounts[i];
            }

            unchecked {
                ++i;
            }
        }
    }

    function delegate() external payable {
        // get validators
        Validator[] memory validators = validatorManager.getAllValidators();
        if (validators.length == 0) revert EmptyValidatorSet();

        uint256 evmAmount = msg.value;
        // Transfer HYPE tokens to core
        uint64 hypeTokenIndex = HLConstants.hypeTokenIndex();
        CoreWriterLib.bridgeToCore(hypeTokenIndex, evmAmount);

        // Using data from the `TokenInfo` precompile, convert EVM amount to core decimals for staking operations
        uint64 coreAmount = HLConversions.evmToWei(hypeTokenIndex, evmAmount);
        // transfer from core to staking balance
        CoreWriterLib.depositStake(coreAmount);

        // get validator addresses array and the amount to stake to that validator
        (
            address[] memory validatorAddresses,
            uint64[] memory amounts
        ) = calculateStakeDistribution(coreAmount, validators);

        uint256 totalValidators = validatorAddresses.length;

        for (uint256 i = 0; i < totalValidators; i++) {
            CoreWriterLib.delegateToken(
                validatorAddresses[i],
                amounts[i],
                false
            );
        }
    }

    function undelegate(uint64 coreAmount) external {
        // get validators
        Validator[] memory validators = validatorManager.getAllValidators();
        if (validators.length == 0) revert EmptyValidatorSet();

        // get validator addresses array and the amount to stake to that validator
        (
            address[] memory validatorAddresses,
            uint64[] memory amounts
        ) = calculateStakeDistribution(coreAmount, validators);

        uint256 totalValidators = validatorAddresses.length;

        for (uint256 i = 0; i < totalValidators; i++) {
            // Undelegate tokens from the validator
            CoreWriterLib.delegateToken(
                validators[0].validator,
                amounts[i],
                true
            );
        }

        // Withdraw the tokens from staking balance to core balances
        CoreWriterLib.withdrawStake(coreAmount);
    }

    function delegationSummary()
        external
        view
        returns (DelegatorSummary memory)
    {
        PrecompileLib.DelegatorSummary memory summary = PrecompileLib
            .delegatorSummary(address(this));
        return
            DelegatorSummary({
                delegated: summary.delegated,
                undelegated: summary.undelegated,
                totalPendingWithdrawal: summary.totalPendingWithdrawal,
                nPendingWithdrawals: summary.nPendingWithdrawals
            });
    }
}
