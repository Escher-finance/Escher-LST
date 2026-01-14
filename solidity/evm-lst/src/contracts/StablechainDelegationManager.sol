// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.28;

import {IERC20, ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {IDelegationManager} from "../interfaces/IDelegationManager.sol";
import {IValidatorSetManager} from "../interfaces/IValidatorSetManager.sol";
import {IStableStaking} from "../interfaces/IStableStaking.sol";
import {IStableDistribution} from "../interfaces/IStableDistribution.sol";
import {Validator, DelegatorSummary} from "../models/Type.sol";
import {CoreWriterLib, HLConstants, HLConversions, PrecompileLib} from "@hyper-evm-lib/src/CoreWriterLib.sol";
import "@openzeppelin-upgradeable/contracts/proxy/utils/Initializable.sol";
import "@openzeppelin-upgradeable/contracts/access/Ownable2StepUpgradeable.sol";
import "@openzeppelin-upgradeable/contracts/proxy/utils/UUPSUpgradeable.sol";
import "@openzeppelin-upgradeable/contracts/utils/PausableUpgradeable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin-upgradeable/contracts/access/AccessControlUpgradeable.sol";
import "@openzeppelin/contracts/utils/math/SafeCast.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

contract StablechainDelegationManager is
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

    address constant PRECOMPILED_STAKING = 0x0000000000000000000000000000000000000800;
    address constant PRECOMPILED_DISTRIBUTION = 0x0000000000000000000000000000000000000801;

    IStableStaking staking;
    IStableDistribution distribution;

    IERC20 asset;
    ERC20 share;

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    function initialize(address owner, address _validatorManager, address _asset, address _share) external initializer {
        // Checks that the initialOwner address is not zero.
        require(owner != address(0), "owner zero address");
        require(_validatorManager != address(0), "validator manager zero address");
        require(_asset != address(0), "_asset zero address");
        require(_share != address(0), "_share zero address");

        __Ownable_init(owner);
        __AccessControl_init();
        require(_grantRole(DEFAULT_ADMIN_ROLE, owner), "failed to grant admin role");
        validatorManager = IValidatorSetManager(_validatorManager);
        staking = IStableStaking(PRECOMPILED_STAKING);
        distribution = IStableDistribution(PRECOMPILED_DISTRIBUTION);
        asset = IERC20(_asset);
        share = ERC20(_share);
    }

    function _authorizeUpgrade(address) internal override onlyOwner {}

    function setLiquidStakingManager(address manager) external onlyOwner {
        require(manager != address(0), "manager zero address");
        require(_grantRole(MANAGER_ROLE, manager), "failed to grant manager role");
    }

    function active() external view returns (bool) {
        return true;
    }

    /**
     * @notice Calculate stake distribution for a given amount
     * @param _amount Total amount to distribute
     * @return addresses Array of validator addresses
     * @return amounts Array of amounts to stake to each validator
     */
    function calculateStakeDistribution(uint256 _amount, Validator[] memory validators)
        internal
        view
        returns (address[] memory addresses, uint256[] memory amounts)
    {
        uint64 totalWeight = validatorManager.getTotalWeight();
        uint256 length = validators.length;
        if (length == 0) revert EmptyValidatorSet();

        addresses = new address[](length);
        amounts = new uint256[](length);

        uint256 distributed = 0;

        for (uint64 i = 0; i < length;) {
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

    function delegate(uint256 amount) external payable nonReentrant {
        require(hasRole(MANAGER_ROLE, msg.sender), "Caller is not a manager");

        // transfer required asset to delegate
        SafeERC20.safeTransfer(asset, address(this), amount);

        // get validators
        Validator[] memory validators = validatorManager.getAllValidators();
        if (validators.length == 0) revert EmptyValidatorSet();

        uint256 coinAmount = msg.value;
        address delegatorAddress = address(this);

        // get validator addresses array and the amount to stake to that validator
        (address[] memory validatorAddresses, uint256[] memory amounts) =
            calculateStakeDistribution(coinAmount, validators);

        uint256 totalValidators = validatorAddresses.length;
        bool successDelegation = true;

        for (uint256 i = 0; i < totalValidators; i++) {
            //delegate to validator according to weight
            successDelegation = staking.delegate(delegatorAddress, validatorAddresses[i], amounts[i]);
            if (!successDelegation) {
                break;
            }
        }

        if (!successDelegation) {
            revert FailedDelegation();
        }

        emit Delegated(msg.sender, amount);
    }

    function undelegate(uint256 amount) external nonReentrant {
        require(hasRole(MANAGER_ROLE, msg.sender), "Caller is not a manager");

        // transfer required share/liquid staking token to undelegate
        SafeERC20.safeTransfer(share, address(this), amount);

        // get validators
        Validator[] memory validators = validatorManager.getAllValidators();
        if (validators.length == 0) revert EmptyValidatorSet();

        // get validator addresses array and the amount to stake to that validator
        (address[] memory validatorAddresses, uint256[] memory amounts) =
            calculateStakeDistribution(uint256(amount), validators);

        uint256 totalValidators = validatorAddresses.length;
        address delegatorAddress = address(this);

        bool successUndelegation = true;
        for (uint256 i = 0; i < totalValidators; i++) {
            // undelegate from validator according to weight
            successUndelegation = staking.undelegate(delegatorAddress, validatorAddresses[i], amounts[i]);
            if (!successUndelegation) {
                break;
            }
        }

        if (!successUndelegation) {
            revert FailedUndelegation();
        }

        emit Undelegated(msg.sender, amount);
    }

    function delegationSummary() external view returns (DelegatorSummary memory) {
        address delegatorAddress = address(this);

        Validator[] memory validators = validatorManager.getAllValidators();
        uint256 totalValidators = validators.length;

        // get total delegated from this contract
        uint256 delegated = 0;
        for (uint256 i = 0; i < totalValidators; i++) {
            (uint256 shares, IStableStaking.Coin memory balance) =
                staking.delegation(delegatorAddress, validators[i].validator);
            delegated += balance.amount;
        }

        (IStableDistribution.DelegationDelegatorReward[] memory rewards, IStableDistribution.DecCoin[] memory total) =
            distribution.delegationTotalRewards(delegatorAddress);

        uint256 _rewards = total[0].amount;

        return DelegatorSummary({
            delegated: SafeCast.toUint64(delegated),
            undelegated: 0,
            totalPendingWithdrawal: 0,
            nPendingWithdrawals: 0,
            rewards: SafeCast.toUint64(_rewards)
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
        uint64 totalDelegated = 1;

        if (totalDelegated == 0) return; // Nothing to redelegate

        // Get current delegations
        PrecompileLib.Delegation[] memory currentDelegations = PrecompileLib.delegations(address(this));

        // Get new validators
        Validator[] memory newValidators = validatorManager.getAllValidators();
        if (newValidators.length == 0) revert EmptyValidatorSet();

        uint64 newTotalWeight = validatorManager.getTotalWeight();

        // Calculate target amounts for each new validator
        uint64[] memory targetAmounts = new uint64[](newValidators.length);
        uint64 distributed = 0;

        for (uint256 i = 0; i < newValidators.length; i++) {
            if (i == newValidators.length - 1) {
                // Last validator gets remaining amount to handle rounding
                targetAmounts[i] = totalDelegated - distributed;
            } else {
                targetAmounts[i] = (totalDelegated * newValidators[i].weight) / newTotalWeight;
                distributed += targetAmounts[i];
            }
        }

        // First pass: undelegate from validators not in new set or with excess
        for (uint256 i = 0; i < currentDelegations.length; i++) {
            address validator = currentDelegations[i].validator;
            uint64 currentAmount = currentDelegations[i].amount;

            // Find if this validator is in the new set and get its target
            uint64 targetAmount = 0;
            bool isInNewSet = false;

            for (uint256 j = 0; j < newValidators.length; j++) {
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
        for (uint256 i = 0; i < newValidators.length; i++) {
            address validator = newValidators[i].validator;
            uint64 targetAmount = targetAmounts[i];

            // Find current amount for this validator
            uint64 currentAmount = 0;
            for (uint256 j = 0; j < currentDelegations.length; j++) {
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

    function receiveBatch(uint256 batchAssets) external nonReentrant {
        require(hasRole(MANAGER_ROLE, msg.sender), "Caller is not a manager");

        // Transfer unbonded assets to liquid staking manager
        (bool success,) = payable(msg.sender).call{value: batchAssets}("");
        require(success, "transfer failed");

        emit BatchReceived(batchAssets);
    }

    function moveBatch(uint256 batchAssets) external nonReentrant {
        require(hasRole(MANAGER_ROLE, msg.sender), "Caller is not a manager");

        // Transfer unbonded assets to liquid staking manager
        (bool success,) = payable(msg.sender).call{value: batchAssets}("");
        require(success, "transfer failed");

        emit BatchMoved(batchAssets);
    }
}
