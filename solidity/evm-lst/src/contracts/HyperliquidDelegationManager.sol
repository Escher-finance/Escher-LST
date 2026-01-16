// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.28;

import {IDelegationManager} from "../interfaces/IDelegationManager.sol";
import {IValidatorSetManager} from "../interfaces/IValidatorSetManager.sol";
import {Validator, DelegatorSummary} from "../models/Type.sol";
import {CoreWriterLib, HLConstants, HLConversions, PrecompileLib} from "@hyper-evm-lib/src/CoreWriterLib.sol";
import "@openzeppelin-upgradeable/contracts/proxy/utils/Initializable.sol";
import "@openzeppelin-upgradeable/contracts/access/Ownable2StepUpgradeable.sol";
import "@openzeppelin-upgradeable/contracts/proxy/utils/UUPSUpgradeable.sol";
import "@openzeppelin-upgradeable/contracts/utils/PausableUpgradeable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin-upgradeable/contracts/access/AccessControlUpgradeable.sol";
import "@openzeppelin/contracts/utils/math/SafeCast.sol";

contract HyperliquidDelegationManager is
    IDelegationManager,
    Initializable,
    UUPSUpgradeable,
    AccessControlUpgradeable,
    Ownable2StepUpgradeable,
    PausableUpgradeable,
    ReentrancyGuard
{
    IValidatorSetManager validatorManager;
    bytes32 public constant MANAGER_ROLE = keccak256("MANAGER_ROLE");
    uint256 public constant CORE_TO_EVM = 10 ** 10;

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    function initialize(address owner, address validatorManagerAddr) external initializer {
        // Checks that the initialOwner address is not zero.
        require(owner != address(0), "owner zero address");
        require(validatorManagerAddr != address(0), "validator manager zero address");
        __Ownable_init(owner);
        __AccessControl_init();
        require(_grantRole(DEFAULT_ADMIN_ROLE, owner), "failed to grant admin role");
        validatorManager = IValidatorSetManager(validatorManagerAddr);
    }

    function _authorizeUpgrade(address) internal override onlyOwner {}

    function setLiquidStakingManager(address manager) external onlyOwner {
        require(manager != address(0), "manager zero address");
        require(_grantRole(MANAGER_ROLE, manager), "failed to grant manager role");
    }

    function active() external view returns (bool) {
        // if no validators, set delegation manager active to false
        Validator[] memory newValidators = validatorManager.getAllValidators();
        if (newValidators.length == 0) return false;
        return PrecompileLib.coreUserExists(address(this));
    }

    /**
     * @notice Calculate stake distribution for a given amount
     * @param delegateAmount Total amount to distribute
     * @return addresses Array of validator addresses
     * @return amounts Array of amounts to stake to each validator
     */
    function calculateStakeDistribution(uint64 delegateAmount, Validator[] memory validators)
        internal
        view
        returns (address[] memory addresses, uint64[] memory amounts)
    {
        uint64 totalWeight = validatorManager.getTotalWeight();
        uint256 totalValidators = validators.length;
        if (totalValidators == 0) revert EmptyValidatorSet();

        addresses = new address[](totalValidators);
        amounts = new uint64[](totalValidators);

        uint64 distributed = 0;

        for (uint64 i = 0; i < totalValidators;) {
            Validator memory v = validators[i];

            addresses[i] = v.validator;

            // Last validator gets remaining amount to handle rounding
            if (i == totalValidators - 1) {
                amounts[i] = delegateAmount - distributed;
            } else {
                amounts[i] = (delegateAmount * v.weight) / totalWeight;
                distributed += amounts[i];
            }

            unchecked {
                ++i;
            }
        }
    }

    function delegate(uint256 amount) external payable nonReentrant {
        require(hasRole(MANAGER_ROLE, msg.sender), "Caller is not a manager");
        require(amount == msg.value, "amount should be same as the fund");

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
        (address[] memory validatorAddresses, uint64[] memory amounts) =
            calculateStakeDistribution(coreAmount, validators);

        uint256 totalValidators = validatorAddresses.length;

        for (uint256 i = 0; i < totalValidators; i++) {
            CoreWriterLib.delegateToken(validatorAddresses[i], amounts[i], false);
        }

        emit Delegated(msg.sender, amount);
    }

    function undelegate(uint256 coreAmount) external nonReentrant {
        require(hasRole(MANAGER_ROLE, msg.sender), "Caller is not a manager");
        // get validators
        Validator[] memory validators = validatorManager.getAllValidators();
        if (validators.length == 0) revert EmptyValidatorSet();

        uint64 amount = SafeCast.toUint64(coreAmount);
        // get validator addresses array and the amount to stake to that validator
        (address[] memory validatorAddresses, uint64[] memory amounts) = calculateStakeDistribution(amount, validators);

        uint256 totalValidators = validatorAddresses.length;

        for (uint256 i = 0; i < totalValidators; i++) {
            // Undelegate tokens from the validator
            CoreWriterLib.delegateToken(validatorAddresses[i], amounts[i], true);
            // Withdraw the tokens from staking balance to core balances
            CoreWriterLib.withdrawStake(amounts[i]);
        }

        emit Undelegated(msg.sender, amount);
    }

    function delegationSummary() external view returns (DelegatorSummary memory) {
        PrecompileLib.DelegatorSummary memory summary = PrecompileLib.delegatorSummary(address(this));
        return DelegatorSummary({
            delegated: summary.delegated,
            undelegated: summary.undelegated,
            totalPendingWithdrawal: summary.totalPendingWithdrawal,
            nPendingWithdrawals: summary.nPendingWithdrawals,
            rewards: 0
        });
    }

    function updateValidators(address[] calldata newValidators, uint64[] calldata newWeights)
        external
        nonReentrant
        onlyOwner
    {
        // update validators with new weights
        validatorManager.updateValidators(newValidators, newWeights);

        // redelegate to new validators set
        _redelegate();

        emit ValidatorsUpdated(newValidators, newWeights);
    }

    /**
     * @notice Redelegates tokens according to the new validator set distribution
     * @dev This function adjusts delegations to match the new weight distribution
     */
    function _redelegate() internal {
        // Get total delegated amount
        PrecompileLib.DelegatorSummary memory summary = PrecompileLib.delegatorSummary(address(this));
        uint64 totalDelegated = summary.delegated;

        if (totalDelegated == 0) return; // Nothing to redelegate

        // Get current delegations
        PrecompileLib.Delegation[] memory currentDelegations = PrecompileLib.delegations(address(this));
        uint256 totalDelegations = currentDelegations.length;

        // Get new validators
        Validator[] memory newValidators = validatorManager.getAllValidators();
        if (newValidators.length == 0) revert EmptyValidatorSet();

        uint64 newTotalWeight = validatorManager.getTotalWeight();

        // Calculate target amounts for each new validator
        uint64[] memory targetAmounts = new uint64[](newValidators.length);
        uint64 distributed = 0;

        uint256 newValidatorsLength = newValidators.length;
        for (uint256 i = 0; i < newValidatorsLength; i++) {
            if (i == newValidatorsLength - 1) {
                // Last validator gets remaining amount to handle rounding
                targetAmounts[i] = totalDelegated - distributed;
            } else {
                targetAmounts[i] = (totalDelegated * newValidators[i].weight) / newTotalWeight;
                distributed += targetAmounts[i];
            }
        }

        // First pass: undelegate from validators not in new set or with excess
        for (uint256 i = 0; i < totalDelegations; i++) {
            address validator = currentDelegations[i].validator;
            uint64 currentAmount = currentDelegations[i].amount;

            // Find if this validator is in the new set and get its target
            uint64 targetAmount = 0;
            bool isInNewSet = false;

            for (uint256 j = 0; j < newValidatorsLength; j++) {
                if (newValidators[j].validator == validator) {
                    isInNewSet = true;
                    targetAmount = targetAmounts[j];
                    break;
                }
            }

            if (!isInNewSet) {
                // Undelegate everything from this validator
                if (currentAmount > 0) {
                    CoreWriterLib.delegateToken(validator, currentAmount, true);
                }
            } else if (currentAmount > targetAmount) {
                // Undelegate the excess
                CoreWriterLib.delegateToken(validator, currentAmount - targetAmount, true);
            }
        }

        // Second pass: delegate to validators that need more
        for (uint256 i = 0; i < newValidatorsLength; i++) {
            address validator = newValidators[i].validator;
            uint64 targetAmount = targetAmounts[i];

            // Find current amount for this validator
            uint64 currentAmount = 0;

            for (uint256 j = 0; j < totalDelegations; j++) {
                if (currentDelegations[j].validator == validator) {
                    currentAmount = currentDelegations[j].amount;
                    break;
                }
            }

            if (currentAmount < targetAmount) {
                // Delegate the difference
                CoreWriterLib.delegateToken(validator, targetAmount - currentAmount, false);
            }
        }
    }

    function moveBatch(uint256 batchAssets) external nonReentrant {
        require(hasRole(MANAGER_ROLE, msg.sender), "Caller is not a manager");

        // Transfer from spot balance to evm
        uint64 hypeTokenIndex = HLConstants.hypeTokenIndex();
        CoreWriterLib.bridgeToEvm(hypeTokenIndex, batchAssets, true);

        emit BatchMoved(batchAssets);
    }

    function receiveBatch(uint256 batchAssets) external nonReentrant {
        require(hasRole(MANAGER_ROLE, msg.sender), "Caller is not a manager");

        // Transfer unbonded assets to liquid staking manager
        (bool success,) = payable(msg.sender).call{value: batchAssets}("");
        require(success, "transfer to receive batch failed");

        emit BatchReceived(batchAssets);
    }

    receive() external payable {}
}
