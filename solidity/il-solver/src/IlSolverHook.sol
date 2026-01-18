// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {BaseHook, Hooks, IPoolManager, ModifyLiquidityParams, BalanceDelta} from "v4-periphery/src/utils/BaseHook.sol";
import {BalanceDeltaLibrary} from "@uniswap/v4-core/src/types/BalanceDelta.sol";
import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {Ownable2Step, Ownable} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {IlSolverMath} from "./core/EscherMath.sol";
import {IAaveOracle} from "aavev3/interfaces/IAaveOracle.sol";
import {DataTypes} from "aavev3/protocol/libraries/types/DataTypes.sol";
import {IL2Pool as IL2PoolOriginal} from "aavev3/interfaces/IL2Pool.sol";
import {IPool} from "aavev3/interfaces/IPool.sol";
import {IWETH} from "@common/IWETH.sol";
import {ReserveConfiguration} from "aavev3/protocol/libraries/configuration/ReserveConfiguration.sol";
import {IlSolverMath} from "./core/EscherMath.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IERC20Metadata} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import {SafeCast} from "@openzeppelin/contracts/utils/math/SafeCast.sol";

using ReserveConfiguration for DataTypes.ReserveConfigurationMap;
using SafeCast for int128;

interface IL2Pool is IL2PoolOriginal, IPool {}

interface IMsgSender {
    function msgSender() external view returns (address);
}

struct UserData {
    uint256 collateralAmountNeeded;
    uint256 iterations;
    uint256 totalBorrowedToken;
    uint256 ltvUsed;
    uint256[] borrowedAmounts;
    bool done;
}

contract IlSolverHook is BaseHook, Ownable2Step {
    mapping(address => bool) public verifiedRouters;
    mapping(address => bool) public users;
    mapping(address => UserData) public usersData;

    IAaveOracle public aaveOracle;
    IL2Pool public aavePool;

    // The borrowed asset
    IWETH public immutable WETH;
    // The collateral asset (e.g. USDC)
    IERC20 public immutable COLLATERAL;

    constructor(
        address _owner,
        IPoolManager _poolManager,
        IWETH _weth,
        IERC20 _collateral,
        IL2Pool _aavePool,
        IAaveOracle _aaveOracle
    ) BaseHook(_poolManager) Ownable(_owner) {
        users[_owner] = true;
        WETH = _weth;
        COLLATERAL = _collateral;
        aavePool = _aavePool;
        aaveOracle = _aaveOracle;
    }

    error TrackerHook_verifiedRouterMissingMsgSender(address router);

    /// @dev Original event from `IPoolManager` with changed name
    /// @notice Emitted when a liquidity position is modified
    /// @param id The abi encoded hash of the pool key struct for the pool that was modified
    /// @param sender The address that modified the pool
    /// @param tickLower The lower tick of the position
    /// @param tickUpper The upper tick of the position
    /// @param liquidityDelta The amount of liquidity that was added or removed
    /// @param salt The extra data to make positions unique
    event TrackedModifyLiquidity(
        bytes32 indexed id,
        address indexed sender,
        int24 tickLower,
        int24 tickUpper,
        int256 liquidityDelta,
        bytes32 salt
    );

    event AddLiquidityData(
        address indexed sender,
        uint256 borrowedAmountNeeded,
        uint256 borrowedTokenUsdPrice,
        uint256 ltv,
        uint256 collateralAmountNeeded
    );

    function getHookPermissions() public pure override returns (Hooks.Permissions memory) {
        return Hooks.Permissions({
            beforeInitialize: false,
            afterInitialize: false,
            beforeAddLiquidity: false,
            afterAddLiquidity: true,
            beforeRemoveLiquidity: false,
            afterRemoveLiquidity: false,
            beforeSwap: false,
            afterSwap: false,
            beforeDonate: false,
            afterDonate: false,
            beforeSwapReturnDelta: false,
            afterSwapReturnDelta: false,
            afterAddLiquidityReturnDelta: false,
            afterRemoveLiquidityReturnDelta: false
        });
    }

    /**
     * @notice Gets the actual sender by calling `.msgSender` from the verified router
     * @return realSender Actual sender
     */
    function _getRealSender(address sender) internal view returns (address realSender) {
        if (verifiedRouters[sender]) {
            try IMsgSender(sender).msgSender() returns (address s) {
                realSender = s;
            } catch {
                revert TrackerHook_verifiedRouterMissingMsgSender(sender);
            }
        }
    }

    /**
     * @notice Toggle IMsgSender as a verified router
     */
    function toggleVerifiedRouter(IMsgSender router) public onlyOwner {
        address routerAddr = address(router);
        verifiedRouters[routerAddr] = !verifiedRouters[routerAddr];
    }

    /**
     * @notice Toggle address as an Il Solver user
     */
    function toggleUser(address user) public onlyOwner {
        users[user] = !users[user];
    }

    /**
     * @notice This function returns the `price` in 18 decimals
     * @return price Current price of a given `asset` from the Aave Oracle
     */
    function aaveOraclePrice(address asset) public view returns (uint256 price) {
        price = aaveOracle.getAssetPrice(asset) * 1e10;
    }

    /**
     * @notice This function returns the `ltv` in 16 decimals
     * @return ltv Loan-to-value ratio of the `collateral` asset
     */
    function aavev3Ltv() public view returns (uint256 ltv) {
        DataTypes.ReserveConfigurationMap memory map = aavePool.getConfiguration(address(COLLATERAL));
        ltv = map.getLtv() * 1e14;
    }

    /**
     * @notice Custom Il Solver logic only applies to calls made from `s_users` and `s_verifiedRouters`
     */
    function _afterAddLiquidity(
        address sender,
        PoolKey calldata,
        ModifyLiquidityParams calldata,
        BalanceDelta delta,
        BalanceDelta,
        bytes calldata
    ) internal override returns (bytes4, BalanceDelta) {
        bytes4 selector = BaseHook.afterAddLiquidity.selector;
        address realSender = _getRealSender(sender);

        if (realSender == address(0) || !users[realSender]) {
            return (selector, delta);
        }

        int128 delta0 = BalanceDeltaLibrary.amount0(delta);
        uint256 borrowedAmountNeeded = delta0 < 0 ? (-delta0).toUint256() : 0;
        uint256 borrowedTokenUsdPrice = aaveOraclePrice(address(WETH));
        uint256 ltv = aavev3Ltv();

        uint8 borrowedTokenDecimals = 18;
        uint8 collateralTokenDecimals = IERC20Metadata(address(COLLATERAL)).decimals();
        (
            uint256 collateralAmountNeeded,
            uint256 iterations,
            uint256 totalBorrowedToken,
            uint256 ltvUsed,
            uint256[] memory borrowedAmounts
        ) = IlSolverMath.calculateCollateralAmount(
            borrowedAmountNeeded, borrowedTokenUsdPrice, borrowedTokenDecimals, collateralTokenDecimals, ltv
        );

        UserData memory senderData = UserData({
            collateralAmountNeeded: collateralAmountNeeded,
            iterations: iterations,
            totalBorrowedToken: totalBorrowedToken,
            ltvUsed: ltvUsed,
            borrowedAmounts: borrowedAmounts,
            done: false
        });

        usersData[realSender] = senderData;

        emit AddLiquidityData(realSender, borrowedAmountNeeded, borrowedTokenUsdPrice, ltv, collateralAmountNeeded);

        return (selector, delta);
    }
}
