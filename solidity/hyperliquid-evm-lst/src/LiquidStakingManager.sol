// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {IERC20, ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {ILiquidStakingManager} from "./interfaces/ILiquidStakingManager.sol";
import {IDelegationManager} from "./interfaces/IDelegationManager.sol";
import {Lst} from "./tokens/Lst.sol";
import {Config, Liquidity} from "./models/State.sol";

import "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts-upgradeable/access/Ownable2StepUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

contract LiquidStakingManager is
    ILiquidStakingManager,
    Initializable,
    UUPSUpgradeable,
    Ownable2StepUpgradeable,
    PausableUpgradeable,
    ReentrancyGuard
{
    Lst share;
    IDelegationManager public delegationManager;

    Config private s_config;
    Liquidity private s_liquidity;

    uint256 public constant SCALING_FACTOR = 10 ** 18;

    // Required by UUPSUpgradeable - only owner can upgrade
    function _authorizeUpgrade(
        address newImplementation
    ) internal override onlyOwner {}

    constructor() {
        _disableInitializers();
    }

    function initialize(
        address initialOwner,
        address lstAddress
    ) public initializer {
        // Checks that the initialOwner address is not zero.
        require(initialOwner != address(0), "zero address");
        __Ownable_init(initialOwner);
        share = Lst(lstAddress);
        s_config = Config({minBondAmount: 1000, minUnbondAmount: 1000});
        s_liquidity = Liquidity({totalDelegated: 0, totalLst: 0});
    }

    function acceptOwnershipTransfer() external onlyOwner {
        share.acceptOwnership(); // Calls Ownable2Step's acceptOwnership
    }

    function setDelegationManager(address _delegate) external onlyOwner {
        delegationManager = IDelegationManager(_delegate);
    }

    function getConfig() external view returns (Config memory) {
        return s_config;
    }

    function getLiquidity() external view returns (Liquidity memory) {
        return s_liquidity;
    }

    function getLst() external view returns (Lst) {
        return share;
    }

    function getDelegationManager() external view returns (address) {
        return address(delegationManager);
    }

    /**
     * @notice function to stake assets and receive liquid staking tokens in exchange
     * @param _assets amount of the asset token
     */
    function bond(uint256 _assets, address _recipient) external payable {
        // checks that the deposited amount is greater than zero.
        require(
            _assets > s_config.minBondAmount,
            "bond should be more than min amount"
        );
        // Checks that the _receiver address is not zero.
        require(_recipient != address(0), "recipient zero address");

        // Checks that the delegationManager address is not zero.
        require(
            address(delegationManager) != address(0),
            "delegationManager zero address"
        );

        // calculate how much shares from the assets
        uint256 shares = _convertToShares(_assets);

        // mint the liquid staking token and send to recipient
        share.mint(_recipient, shares);

        emit Bond(msg.sender, _assets, _recipient);
    }

    /**
     * @notice function to calculate how much shares from amount of assets base on exchange rate
     * @param assets amount of the asset token
     */
    function _convertToShares(uint256 assets) internal view returns (uint256) {
        return (bondRate() * assets) / SCALING_FACTOR;
    }

    function totalAssets() public view returns (uint256) {
        uint256 totalReward = 0; // TODO: Replace with query to chain
        return s_liquidity.totalDelegated + totalReward;
    }

    /**
     * @notice function to calculate the rate to get shares token/lst from assets token
     */
    function bondRate() public view returns (uint256) {
        if ((s_liquidity.totalLst == 0) || (s_liquidity.totalDelegated == 0)) {
            return 1 * SCALING_FACTOR;
        }
        return (totalAssets() * SCALING_FACTOR) / s_liquidity.totalLst;
    }

    /**
     * @notice function to calculate the rate to get assets token from shares token/lst
     */
    function unbondRate() public view returns (uint256) {
        if ((s_liquidity.totalLst == 0) || (s_liquidity.totalDelegated == 0)) {
            return 1 * SCALING_FACTOR;
        }
        return (s_liquidity.totalLst * SCALING_FACTOR) / totalAssets();
    }

    /**
     * @notice Function to allow msg.sender to request to unbond of their staked asset
     * @param _shares amount of shares the user wants to convert
     * @param _recipient address of the user who will receive the assets
     */
    function unbondRequest(uint256 _shares, address _recipient) external {
        // checks that the deposited amount is greater than zero.
        require(
            _shares > s_config.minUnbondAmount,
            "unbond should be more than min unbond amount"
        );
        // Checks that the _receiver address is not zero.
        require(_recipient != address(0), "recipient zero address");

        // Checks that the delegationManager address is not zero.
        require(
            address(delegationManager) != address(0),
            "delegation Manager zero address"
        );

        // transfer asset to this contract
        share.transferFrom(msg.sender, address(this), _shares);
        emit UnbondRequest(msg.sender, _shares, _recipient);
    }
}
