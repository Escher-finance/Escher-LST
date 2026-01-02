// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

// FIXME: dedupe to a single v4-core instead of importing v4-periphery's v4-core types

import {Ownable2Step, Ownable} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {IPositionManager as IPositionManagerOriginal} from "univ4-periphery/interfaces/IPositionManager.sol";
import {
    IV4Router,
    PoolKey as PeripheryPoolKey,
    Currency as PeripheryCurrency
} from "univ4-periphery/interfaces/IV4Router.sol";
import {IHooks} from "univ4-periphery/libraries/PathKey.sol";
import {Actions} from "univ4-periphery/libraries/Actions.sol";
import {PoolKey} from "univ4-core/types/PoolKey.sol";
import {IPoolManager} from "univ4-core/interfaces/IPoolManager.sol";
import {Currency, CurrencyLibrary} from "univ4-core/types/Currency.sol";
import {IImmutableState} from "univ4-periphery/interfaces/IImmutableState.sol";
import {IL2Pool as IL2PoolOriginal} from "aavev3/interfaces/IL2Pool.sol";
import {IAaveOracle} from "aavev3/interfaces/IAaveOracle.sol";
import {IPool} from "aavev3/interfaces/IPool.sol";
import {L2Encoder} from "aavev3/helpers/L2Encoder.sol";
import {DataTypes} from "aavev3/protocol/libraries/types/DataTypes.sol";
import {ReserveConfiguration} from "aavev3/protocol/libraries/configuration/ReserveConfiguration.sol";
import {IPoolDataProvider} from "aavev3/interfaces/IPoolDataProvider.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IlSolverMath} from "./core/EscherMath.sol";
import {IWETH} from "@common/IWETH.sol";

using CurrencyLibrary for Currency;
using SafeERC20 for IERC20;
using ReserveConfiguration for DataTypes.ReserveConfigurationMap;

interface IPositionManager is IPositionManagerOriginal {
    function permit2() external view returns (address);
}

interface IL2Pool is IL2PoolOriginal, IPool {}

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

contract IlSolver is Ownable2Step {
    // The borrowed asset
    IWETH public immutable WETH;
    // The collateral asset (e.g. USDC)
    IERC20 public immutable collateral;

    IPermit2 permit2;

    // Uniswap V4

    IPositionManager public uniPosm;
    IPoolManager uniPoolManager;
    IUniversalRouter uniRouter;
    // Must have `currency0` set to ETH and `currency1` set to `collateral`
    PoolKey public uniPoolKey;
    uint256 public uniPositionTokenId;

    // Aave V3

    IL2Pool public aavePool;
    L2Encoder public aaveEncoder;
    IPoolDataProvider public aaveDataProvider;
    IAaveOracle aaveOracle;
    // Whether collateral has been set
    bool public aaveCollateralSet;

    error IlSolver_wrongETHValueSent(uint256 needed, uint256 got);
    error IlSolver_wrongERC20Allowance(IERC20 token, uint256 needed, uint256 got);

    constructor(
        address _owner,
        IWETH _weth,
        IERC20 _collateral,
        IPositionManager _uniPosm,
        PoolKey memory _uniPoolKey,
        IUniversalRouter _uniRouter,
        IL2Pool _aavePool,
        L2Encoder _aaveEncoder,
        IPoolDataProvider _aaveDataProvider,
        IAaveOracle _aaveOracle
    ) Ownable(_owner) {
        WETH = _weth;

        permit2 = IPermit2(_uniPosm.permit2());
        uniPoolManager = IPoolManager(address(_uniPosm.poolManager()));
        uniPosm = _uniPosm;
        uniRouter = _uniRouter;

        require(_uniPoolKey.currency0.isAddressZero());
        require(Currency.unwrap(_uniPoolKey.currency1) == address(_collateral));

        uniPoolKey = _uniPoolKey;

        (,,,,, bool usageAsCollateralEnabled,,,,) = _aaveDataProvider.getReserveConfigurationData(address(_collateral));
        require(usageAsCollateralEnabled);
        aaveDataProvider = _aaveDataProvider;
        collateral = _collateral;
        aavePool = _aavePool;
        aaveEncoder = _aaveEncoder;
        aaveOracle = _aaveOracle;
    }

    receive() external payable {}

    /**
     * @dev Sets allowances and validates contract's funds to use in Uniswap V4
     * @notice In the case there's not enough ETH (`_amount0Max`), it will attempt to unwrap the right amount of WETH
     */
    modifier univ4AttachFunds(uint128 _amount0Max, uint128 _amount1Max) {
        require(_amount0Max > 0);
        require(_amount1Max > 0);

        uint256 amount0Max = uint256(_amount0Max);
        uint256 amount1Max = uint256(_amount1Max);
        PoolKey memory key = uniPoolKey;
        address _this = address(this);
        address _posm = address(uniPosm);

        // Handle ETH

        uint256 ethBalance = _this.balance;
        uint256 ethNeeded = (ethBalance < amount0Max) ? amount0Max - ethBalance : 0;
        if (ethNeeded > 0) {
            WETH.withdraw(ethNeeded);
        }

        // Handle collateral

        IERC20 t1 = IERC20(Currency.unwrap(key.currency1));
        if (t1.allowance(_this, address(permit2)) < amount1Max) {
            t1.approve(address(permit2), type(uint128).max);
        }
        (uint160 p2Allowance1,,) = permit2.allowance(_this, address(t1), _posm);
        if (p2Allowance1 < amount1Max) {
            permit2.approve(address(t1), _posm, type(uint128).max, type(uint48).max);
        }

        _;
    }

    /**
     * @dev Mints a new Uniswap V4 position given the tick range and `liquidity`
     * @notice Creates a position NFT and stores its token ID in `uniPositionTokenId`
     * @notice Uses contract's funds
     * @return used0 Amount used out of `amount0Max`
     * @return used1 Amount used out of `amount1Max`
     */
    function _univ4LiquidityMint(
        int24 tickLower,
        int24 tickUpper,
        uint256 liquidity,
        uint128 amount0Max,
        uint128 amount1Max
    ) private univ4AttachFunds(amount0Max, amount1Max) returns (uint256 used0, uint256 used1) {
        address _this = address(this);
        PoolKey memory _key = uniPoolKey;

        bytes memory actions =
            abi.encodePacked(uint8(Actions.MINT_POSITION), uint8(Actions.SETTLE_PAIR), uint8(Actions.SWEEP));

        bytes[] memory params = new bytes[](3);
        params[0] = abi.encode(_key, tickLower, tickUpper, liquidity, amount0Max, amount1Max, _this, bytes(""));
        params[1] = abi.encode(_key.currency0, _key.currency1);
        params[2] = abi.encode(CurrencyLibrary.ADDRESS_ZERO, _this);

        uint256 deadline = block.timestamp;
        uint256 positionId = uniPosm.nextTokenId();
        address posm = address(uniPosm);
        IERC20(Currency.unwrap(_key.currency1)).approve(posm, amount1Max);

        uint256 b0Before = _key.currency0.balanceOfSelf();
        uint256 b1Before = _key.currency1.balanceOfSelf();

        uniPosm.modifyLiquidities{value: amount0Max}(abi.encode(actions, params), deadline);

        uniPositionTokenId = positionId;

        uint256 b0After = _key.currency0.balanceOfSelf();
        uint256 b1After = _key.currency1.balanceOfSelf();

        used0 = b0Before + amount0Max - b0After;
        used1 = b1Before + amount1Max - b1After;
    }

    /**
     * @dev Increments the Uniswap V4 position with ID `uniPositionTokenId` with the given `liquidity`
     * @notice Uses contract's funds
     * @return used0 Amount used out of `amount0Max`
     * @return used1 Amount used out of `amount1Max`
     */
    function _univ4LiquidityIncrement(uint256 liquidity, uint128 amount0Max, uint128 amount1Max)
        private
        univ4AttachFunds(amount0Max, amount1Max)
        returns (uint256 used0, uint256 used1)
    {
        address _this = address(this);
        uint256 _tokenId = uniPositionTokenId;
        PoolKey memory _key = uniPoolKey;

        bytes memory actions =
            abi.encodePacked(uint8(Actions.INCREASE_LIQUIDITY), uint8(Actions.SETTLE_PAIR), uint8(Actions.SWEEP));

        bytes[] memory params = new bytes[](3);
        params[0] = abi.encode(_tokenId, liquidity, amount0Max, amount1Max, bytes(""));
        params[1] = abi.encode(_key.currency0, _key.currency1);
        params[2] = abi.encode(CurrencyLibrary.ADDRESS_ZERO, _this);

        uint256 deadline = block.timestamp;
        address posm = address(uniPosm);
        IERC20(Currency.unwrap(_key.currency1)).approve(posm, amount1Max);

        uint256 b0Before = _key.currency0.balanceOfSelf();
        uint256 b1Before = _key.currency1.balanceOfSelf();

        uniPosm.modifyLiquidities{value: amount0Max}(abi.encode(actions, params), deadline);

        uint256 b0After = _key.currency0.balanceOfSelf();
        uint256 b1After = _key.currency1.balanceOfSelf();

        used0 = b0Before + amount0Max - b0After;
        used1 = b1Before + amount1Max - b1After;
    }

    // swaps exact input single
    function _univ4Swap(bool zeroForOne, uint128 amountIn, uint128 minAmountOut) private {
        bytes memory commands = abi.encodePacked(uint8(UniversalRouterCommands.V4_SWAP));
        bytes memory actions =
            abi.encodePacked(uint8(Actions.SWAP_EXACT_IN_SINGLE), uint8(Actions.SETTLE_ALL), uint8(Actions.TAKE_ALL));

        PoolKey memory key = uniPoolKey;
        bytes[] memory params = new bytes[](3);
        PeripheryPoolKey memory _key = PeripheryPoolKey({
            currency0: PeripheryCurrency.wrap(Currency.unwrap(key.currency0)),
            currency1: PeripheryCurrency.wrap(Currency.unwrap(key.currency1)),
            fee: key.fee,
            tickSpacing: key.tickSpacing,
            hooks: IHooks(address(key.hooks))
        });

        params[0] = abi.encode(
            IV4Router.ExactInputSingleParams({
                poolKey: _key,
                zeroForOne: zeroForOne,
                amountIn: amountIn,
                amountOutMinimum: minAmountOut,
                hookData: bytes("")
            })
        );
        params[1] = abi.encode(key.currency0, amountIn);
        params[2] = abi.encode(key.currency1, minAmountOut);

        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(actions, params);
        uint256 deadline = block.timestamp;

        uniRouter.execute(commands, inputs, deadline);
    }

    /**
     * @dev Mints or increments the Uniswap V4 position depending on whether `uniPositionTokenId` is set
     * @notice Uses contract's funds
     * @return used0 Amount used out of `amount0Max`
     * @return used1 Amount used out of `amount1Max`
     */
    function _univ4LiquidityAdd(
        int24 tickLower,
        int24 tickUpper,
        uint256 liquidity,
        uint128 amount0Max,
        uint128 amount1Max
    ) private returns (uint256 used0, uint256 used1) {
        if (uniPositionTokenId == 0) {
            return _univ4LiquidityMint(tickLower, tickUpper, liquidity, amount0Max, amount1Max);
        } else {
            return _univ4LiquidityIncrement(liquidity, amount0Max, amount1Max);
        }
    }

    /**
     * @dev Supplies `collateral` token to Aave V3
     * @notice Uses contract's funds
     * @notice If it's the first deposit it sets the collateral as the reserve token
     */
    function _aavev3Supply(uint256 amount) private {
        if (collateral.allowance(address(this), address(aavePool)) < amount) {
            collateral.approve(address(aavePool), type(uint128).max);
        }
        bytes32 params = aaveEncoder.encodeSupplyParams(address(collateral), amount, 0);
        aavePool.supply(params);

        if (!aaveCollateralSet) {
            aavePool.setUserUseReserveAsCollateral(address(collateral), true);
            aaveCollateralSet = true;
        }
    }

    /**
     * @dev Borrows `WETH` from Aave V3 using supplied `collateral`
     */
    function _aavev3Borrow(uint256 amount) private {
        aavePool.borrow(address(WETH), amount, 2, 0, address(this));
    }

    /**
     * @dev Mints or increments the Uniswap V4 position depending on whether `uniPositionTokenId` is set
     * @dev See internal helper {_univ4LiquidityAdd}
     */
    function univ4LiquidityAdd(
        int24 tickLower,
        int24 tickUpper,
        uint256 liquidity,
        uint128 amount0Max,
        uint128 amount1Max
    ) public onlyOwner returns (uint256 used0, uint256 used1) {
        return _univ4LiquidityAdd(tickLower, tickUpper, liquidity, amount0Max, amount1Max);
    }

    /**
     * @dev Supplies `collateral` token to Aave V3
     * @dev See internal helper {_aavev3Supply}
     */
    function aavev3Supply(uint256 amount) public onlyOwner {
        _aavev3Supply(amount);
    }

    /**
     * @dev Borrows `WETH` from Aave V3 using supplied `collateral`
     * @dev See internal helper {_aavev3Borrow}
     */
    function aavev3Borrow(uint256 amount) public onlyOwner {
        _aavev3Borrow(amount);
    }

    /**
     * @return ltv Loan-to-value ratio of the `collateral` asset
     */
    function aavev3Ltv() public view returns (uint256 ltv) {
        DataTypes.ReserveConfigurationMap memory map = aavePool.getConfiguration(address(collateral));
        ltv = map.getLtv();
    }

    /**
     * @return price Current price of a given `asset` from the Aave Oracle
     */
    function aaveOraclePrice(address asset) public returns (uint256 price) {
        price = aaveOracle.getAssetPrice(asset);
    }
}
