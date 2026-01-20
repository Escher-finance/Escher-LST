// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {BaseHook, Hooks, IPoolManager, ModifyLiquidityParams, BalanceDelta} from "v4-periphery/src/utils/BaseHook.sol";
import {BalanceDeltaLibrary} from "@uniswap/v4-core/src/types/BalanceDelta.sol";
import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {Ownable2Step, Ownable} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {IV4Router} from "@uniswap/v4-periphery/src/interfaces/IV4Router.sol";
import {Actions} from "@uniswap/v4-periphery/src/libraries/Actions.sol";
import {IlSolverMath} from "./core/EscherMath.sol";
import {Currency, CurrencyLibrary} from "@uniswap/v4-core/src/types/Currency.sol";
import {IAaveOracle} from "aavev3/interfaces/IAaveOracle.sol";
import {DataTypes} from "aavev3/protocol/libraries/types/DataTypes.sol";
import {IL2Pool as IL2PoolOriginal} from "aavev3/interfaces/IL2Pool.sol";
import {IPool} from "aavev3/interfaces/IPool.sol";
import {L2Encoder} from "aavev3/helpers/L2Encoder.sol";
import {IWETH} from "@common/IWETH.sol";
import {ReserveConfiguration} from "aavev3/protocol/libraries/configuration/ReserveConfiguration.sol";
import {IlSolverMath} from "./core/EscherMath.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IERC20Metadata} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import {SafeCast} from "@openzeppelin/contracts/utils/math/SafeCast.sol";

using CurrencyLibrary for Currency;
using ReserveConfiguration for DataTypes.ReserveConfigurationMap;
using SafeCast for int128;

interface IL2Pool is IL2PoolOriginal, IPool {}

interface IMsgSender {
    function msgSender() external view returns (address);
}

interface IUniversalRouter {
    function execute(bytes calldata commands, bytes[] calldata inputs, uint256 deadline) external payable;
}

interface IPermit2 {
    function allowance(address user, address token, address spender)
        external
        view
        returns (uint160 amount, uint48 expiration, uint48 nonce);
    function approve(address token, address spender, uint160 amount, uint48 expiration) external;
}

// src: https://github.com/Uniswap/universal-router/blob/main/contracts/libraries/Commands.sol
library UniversalRouterCommands {
    // Masks to extract certain bits of commands
    bytes1 internal constant FLAG_ALLOW_REVERT = 0x80;
    bytes1 internal constant COMMAND_TYPE_MASK = 0x7f;

    // Command Types. Maximum supported command at this moment is 0x3f.
    // The commands are executed in nested if blocks to minimise gas consumption

    // Command Types where value<=0x07, executed in the first nested-if block
    uint256 constant V3_SWAP_EXACT_IN = 0x00;
    uint256 constant V3_SWAP_EXACT_OUT = 0x01;
    uint256 constant PERMIT2_TRANSFER_FROM = 0x02;
    uint256 constant PERMIT2_PERMIT_BATCH = 0x03;
    uint256 constant SWEEP = 0x04;
    uint256 constant TRANSFER = 0x05;
    uint256 constant PAY_PORTION = 0x06;
    // COMMAND_PLACEHOLDER = 0x07;

    // Command Types where 0x08<=value<=0x0f, executed in the second nested-if block
    uint256 constant V2_SWAP_EXACT_IN = 0x08;
    uint256 constant V2_SWAP_EXACT_OUT = 0x09;
    uint256 constant PERMIT2_PERMIT = 0x0a;
    uint256 constant WRAP_ETH = 0x0b;
    uint256 constant UNWRAP_WETH = 0x0c;
    uint256 constant PERMIT2_TRANSFER_FROM_BATCH = 0x0d;
    uint256 constant BALANCE_CHECK_ERC20 = 0x0e;
    // COMMAND_PLACEHOLDER = 0x0f;

    // Command Types where 0x10<=value<=0x20, executed in the third nested-if block
    uint256 constant V4_SWAP = 0x10;
    uint256 constant V3_POSITION_MANAGER_PERMIT = 0x11;
    uint256 constant V3_POSITION_MANAGER_CALL = 0x12;
    uint256 constant V4_INITIALIZE_POOL = 0x13;
    uint256 constant V4_POSITION_MANAGER_CALL = 0x14;
    // COMMAND_PLACEHOLDER = 0x15 -> 0x20

    // Command Types where 0x21<=value<=0x3f
    uint256 constant EXECUTE_SUB_PLAN = 0x21;
    // COMMAND_PLACEHOLDER for 0x22 to 0x3f

    // Command Types where 0x40<=value<=0x5f
    // Reserved for 3rd party integrations
    uint256 constant ACROSS_V4_DEPOSIT_V3 = 0x40;
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
    mapping(address => UserData) usersData;

    IAaveOracle public aaveOracle;
    L2Encoder public aaveEncoder;
    IL2Pool public aavePool;
    // Whether collateral has been set
    bool public aaveCollateralSet;

    // The borrowed asset
    IWETH public immutable WETH;
    // The collateral asset (e.g. USDC)
    IERC20 public immutable COLLATERAL;

    // Must have `currency0` set to ETH and `currency1` set to `collateral`
    // Used in swap
    PoolKey public uniPoolKey;
    IUniversalRouter public uniRouter;

    IPermit2 public permit2;

    constructor(
        address _owner,
        IPoolManager _poolManager,
        IWETH _weth,
        IERC20 _collateral,
        IPermit2 _permit2,
        PoolKey memory _uniPoolKey,
        IUniversalRouter _uniRouter,
        IL2Pool _aavePool,
        L2Encoder _aaveEncoder,
        IAaveOracle _aaveOracle
    ) BaseHook(_poolManager) Ownable(_owner) {
        users[_owner] = true;

        require(Currency.unwrap(_uniPoolKey.currency0) == address(0));
        require(Currency.unwrap(_uniPoolKey.currency1) == address(_collateral));
        uniPoolKey = _uniPoolKey;

        WETH = _weth;
        COLLATERAL = _collateral;
        uniRouter = _uniRouter;
        aavePool = _aavePool;
        aaveOracle = _aaveOracle;
        aaveEncoder = _aaveEncoder;
        permit2 = _permit2;
    }

    receive() external payable {}

    modifier onlyUser() {
        require(users[msg.sender]);
        _;
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

        UserData memory senderData = usersData[realSender];

        if (
            realSender == address(0) || !users[realSender]
                || (senderData.collateralAmountNeeded > 0 && !senderData.done)
        ) {
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

        UserData memory newSenderData = UserData({
            collateralAmountNeeded: collateralAmountNeeded,
            iterations: iterations,
            totalBorrowedToken: totalBorrowedToken,
            ltvUsed: ltvUsed,
            borrowedAmounts: borrowedAmounts,
            done: false
        });

        usersData[realSender] = newSenderData;

        emit AddLiquidityData(realSender, borrowedAmountNeeded, borrowedTokenUsdPrice, ltv, collateralAmountNeeded);

        return (selector, delta);
    }

    /**
     * @dev Supplies `collateral` token to Aave V3
     * @notice Uses contract's funds
     */
    function _aavev3Supply(uint256 amount) private {
        if (COLLATERAL.allowance(address(this), address(aavePool)) < amount) {
            COLLATERAL.approve(address(aavePool), type(uint128).max);
        }

        bytes32 params = aaveEncoder.encodeSupplyParams(address(COLLATERAL), amount, 0);
        aavePool.supply(params);

        if (!aaveCollateralSet) {
            aavePool.setUserUseReserveAsCollateral(address(COLLATERAL), true);
            aaveCollateralSet = true;
        }
    }

    /**
     * @dev Borrows `WETH` from Aave V3 using supplied `collateral`
     */
    function _aavev3Borrow(uint256 amount) private {
        aavePool.borrow(address(WETH), amount, 2, 0, address(this));
    }

    modifier univ4AttachFundsForSwap(bool zeroForOne, uint128 _amountIn) {
        require(_amountIn > 0);

        uint256 amountIn = uint256(_amountIn);
        PoolKey memory key = uniPoolKey;
        address _this = address(this);
        address _router = address(uniRouter);

        // Handle ETH

        if (zeroForOne) {
            uint256 ethBalance = _this.balance;
            uint256 ethNeeded = (ethBalance < amountIn) ? amountIn - ethBalance : 0;
            if (ethNeeded > 0) {
                WETH.withdraw(ethNeeded);
            }
        }

        // Handle collateral

        if (!zeroForOne) {
            IERC20 t1 = IERC20(Currency.unwrap(key.currency1));
            if (t1.allowance(_this, address(permit2)) < amountIn) {
                t1.approve(address(permit2), type(uint128).max);
            }
            (uint160 p2Allowance1,,) = permit2.allowance(_this, address(t1), address(_router));
            if (p2Allowance1 < amountIn) {
                permit2.approve(address(t1), _router, type(uint128).max, type(uint48).max);
            }
        }

        _;
    }

    /**
     * @dev Swaps exact input single from the Uniswap V4 pool with `uniPoolKey`
     * @dev Swaps one of the tokens for the other configured via `zeroForOne`
     * @return actualAmountOut Actual amount returned after the swap (i.e. amount1 returned to the contract if zeroForOne is true)
     */
    function _univ4Swap(bool zeroForOne, uint128 amountIn, uint128 minAmountOut)
        private
        univ4AttachFundsForSwap(zeroForOne, amountIn)
        returns (uint256 actualAmountOut)
    {
        bytes memory commands = abi.encodePacked(uint8(UniversalRouterCommands.V4_SWAP));
        bytes memory actions =
            abi.encodePacked(uint8(Actions.SWAP_EXACT_IN_SINGLE), uint8(Actions.SETTLE_ALL), uint8(Actions.TAKE_ALL));

        PoolKey memory key = uniPoolKey;
        bytes[] memory params = new bytes[](3);

        params[0] = abi.encode(
            IV4Router.ExactInputSingleParams({
                poolKey: key,
                zeroForOne: zeroForOne,
                amountIn: amountIn,
                amountOutMinimum: minAmountOut,
                hookData: bytes("")
            })
        );
        params[1] = abi.encode(zeroForOne ? key.currency0 : key.currency1, amountIn);
        params[2] = abi.encode(zeroForOne ? key.currency1 : key.currency0, minAmountOut);

        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(actions, params);
        uint256 deadline = block.timestamp;

        uint256 bBefore = (zeroForOne) ? key.currency1.balanceOfSelf() : key.currency0.balanceOfSelf();

        uniRouter.execute{value: zeroForOne ? amountIn : 0}(commands, inputs, deadline);

        uint256 bAfter = (zeroForOne) ? key.currency1.balanceOfSelf() : key.currency0.balanceOfSelf();

        actualAmountOut = bAfter - bBefore;
    }

    function loop() public onlyUser {
        UserData memory senderData = usersData[msg.sender];
        require(senderData.collateralAmountNeeded > 0 && !senderData.done);
        uint256 iterations = senderData.iterations / 1e18;

        uint256 collateralDecimals = uint256(IERC20Metadata(address(COLLATERAL)).decimals());
        uint256 wethPrice = aaveOraclePrice(address(WETH));

        uint256 collateralAmount = senderData.collateralAmountNeeded;
        COLLATERAL.transferFrom(msg.sender, address(this), collateralAmount);

        // 1. supply usdc (collateral)
        // 2. borrow weth
        // 3. swap weth with usdc
        for (uint256 i = 0; i < iterations; i++) {
            _aavev3Supply(collateralAmount);
            uint256 currentBorrowAmount = senderData.borrowedAmounts[i];
            _aavev3Borrow(currentBorrowAmount);
            uint256 expectedAmountOut = currentBorrowAmount * wethPrice / (10 ** (36 - collateralDecimals));
            uint256 minAmountOut = expectedAmountOut * 98 / 100; // add some slippage
            collateralAmount = _univ4Swap(true, uint128(currentBorrowAmount), uint128(minAmountOut));
        }

        usersData[msg.sender].done = true;
    }

    function getUserData(address user) public view returns (UserData memory data) {
        data = usersData[user];
    }
}
