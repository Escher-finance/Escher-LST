// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import {Test, console} from "forge-std/Test.sol";
import {Hooks} from "@uniswap/v4-core/src/libraries/Hooks.sol";
import {Actions} from "@uniswap/v4-periphery/src/libraries/Actions.sol";
import {HookMiner} from "@uniswap/v4-periphery/src/utils/HookMiner.sol";
import {IlSolverHook, IWETH, IERC20, IPoolManager, IL2Pool, IAaveOracle} from "../src/IlSolverHook.sol";
import {TickMath} from "@uniswap/v4-core/src/libraries/TickMath.sol";
import {LiquidityAmounts} from "@uniswap/v4-core/test/utils/LiquidityAmounts.sol";
import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {Currency, CurrencyLibrary} from "@uniswap/v4-core/src/types/Currency.sol";
import {IHooks} from "@uniswap/v4-core/src/interfaces/IHooks.sol";
import {IStateView} from "@uniswap/v4-periphery/src/interfaces/IStateView.sol";
import {IPositionManager} from "@uniswap/v4-periphery/src/interfaces/IPositionManager.sol";

interface IPermit2 {
    function allowance(address user, address token, address spender)
        external
        view
        returns (uint160 amount, uint48 expiration, uint48 nonce);
    function approve(address token, address spender, uint160 amount, uint48 expiration) external;
}

contract IlSolverHookTest is Test {
    address usdc = 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913;
    address weth = 0x4200000000000000000000000000000000000006;

    IWETH WETH;
    IERC20 collateral;
    IERC20 reserve;

    IPermit2 permit2;

    IPoolManager uniPoolManager;
    IPositionManager uniPosm;
    PoolKey uniPoolKey;
    IStateView uniStateView;
    // NOTE: this pool is used only for reference to create the new one
    PoolKey _referencePoolKey;

    IL2Pool aavePool;
    IAaveOracle aaveOracle;

    IlSolverHook h;
    address owner;

    function setUp() public {
        vm.createSelectFork("base", 39260000);
        owner = makeAddr("owner");

        _referencePoolKey = PoolKey({
            currency0: CurrencyLibrary.ADDRESS_ZERO,
            currency1: Currency.wrap(usdc),
            fee: 500,
            tickSpacing: 10,
            hooks: IHooks(address(0))
        });

        permit2 = IPermit2(0x000000000022D473030F116dDEE9F6B43aC78BA3);
        uniPoolManager = IPoolManager(0x498581fF718922c3f8e6A244956aF099B2652b2b);
        uniPosm = IPositionManager(0x498581fF718922c3f8e6A244956aF099B2652b2b);
        uniStateView = IStateView(0xA3c0c9b65baD0b08107Aa264b0f3dB444b867A71);
        aavePool = IL2Pool(0xA238Dd80C259a72e81d7e4664a9801593F98d1c5);
        aaveOracle = IAaveOracle(0x2Cc0Fc26eD4563A5ce5e8bdcfe1A2878676Ae156);

        WETH = IWETH(weth);
        collateral = IERC20(usdc);
        reserve = IERC20(aavePool.getReserveAToken(usdc));

        uint160 flags = uint160(Hooks.AFTER_ADD_LIQUIDITY_FLAG);
        bytes memory constructorArgs = abi.encode(owner, uniPoolManager, WETH, collateral, aavePool, aaveOracle);
        (address hookAddress, bytes32 salt) =
            HookMiner.find(address(this), flags, type(IlSolverHook).creationCode, constructorArgs);

        // deploy hook contract
        h = new IlSolverHook{salt: salt}(owner, uniPoolManager, WETH, collateral, aavePool, aaveOracle);
        assertEq(address(h), hookAddress);

        // create pool
        uniPoolKey = PoolKey({
            currency0: _referencePoolKey.currency0,
            currency1: _referencePoolKey.currency1,
            fee: _referencePoolKey.fee,
            tickSpacing: _referencePoolKey.tickSpacing,
            hooks: IHooks(hookAddress)
        });

        // fund owner
        deal(owner, 10 ether);
        deal(Currency.unwrap(uniPoolKey.currency1), owner, 50000e6);

        (uint160 sqrtPriceX96,,,) = uniStateView.getSlot0(_referencePoolKey.toId());
        // initialize pool
        uniPoolManager.initialize(uniPoolKey, sqrtPriceX96);
    }

    function test_canCallOracle() public {
        uint256 price = h.aaveOraclePrice(address(WETH));
        assertGt(price, 0);
    }

    function _calculateInputs(uint256 amount0, int24 delta, uint256 slippage)
        private
        view
        returns (int24 tickLower, int24 tickUpper, uint128 liquidity, uint128 amount0Max, uint128 amount1Max)
    {
        (uint160 sqrtPriceX96, int24 tick,,) = uniStateView.getSlot0(uniPoolKey.toId());

        int24 tickSpacing = uniPoolKey.tickSpacing;
        tickLower = ((tick - delta) / tickSpacing) * tickSpacing;
        tickUpper = ((tick + delta) / tickSpacing) * tickSpacing;

        uint160 sqrtPriceAX96 = TickMath.getSqrtPriceAtTick(tickLower);
        uint160 sqrtPriceBX96 = TickMath.getSqrtPriceAtTick(tickUpper);
        liquidity = LiquidityAmounts.getLiquidityForAmount0(sqrtPriceAX96, sqrtPriceBX96, amount0);

        (uint256 required0, uint256 required1) =
            LiquidityAmounts.getAmountsForLiquidity(sqrtPriceX96, sqrtPriceAX96, sqrtPriceBX96, liquidity);

        amount0Max = uint128(required0 * (100 + slippage) / 100);
        amount1Max = uint128(required1 * (100 + slippage) / 100);
    }

    function _univ4LiquidityMint(
        address caller,
        int24 tickLower,
        int24 tickUpper,
        uint256 liquidity,
        uint128 amount0Max,
        uint128 amount1Max
    ) private returns (uint256 used0, uint256 used1) {
        PoolKey memory _key = uniPoolKey;
        address posm = address(uniPosm);

        vm.startPrank(caller);

        IERC20(Currency.unwrap(_key.currency1)).approve(posm, amount1Max);

        bytes memory actions = abi.encodePacked(uint8(Actions.MINT_POSITION), uint8(Actions.SETTLE_PAIR));

        bytes[] memory params = new bytes[](2);
        params[0] = abi.encode(_key, tickLower, tickUpper, liquidity, amount0Max, amount1Max, caller, bytes(""));
        params[1] = abi.encode(_key.currency0, _key.currency1);

        uint256 b0Before = _key.currency0.balanceOf(owner);
        uint256 b1Before = _key.currency1.balanceOf(owner);

        uint256 deadline = block.timestamp;
        uint256 valueToPass = _key.currency0.isAddressZero() ? amount0Max : 0;

        uniPosm.modifyLiquidities{value: valueToPass}(abi.encode(actions, params), deadline);

        uint256 b0After = _key.currency0.balanceOf(owner);
        uint256 b1After = _key.currency1.balanceOf(owner);

        vm.stopPrank();

        used0 = b0Before - b0After;
        used1 = b1Before - b1After;
    }

    function test_afterAddLiquidityEvent() public {
        uint256 amount0 = 0.1 ether;
        int24 delta = 488; // 5% in ticks
        uint256 slippage = 10; // 10%
        (int24 tickLower, int24 tickUpper, uint128 liquidity, uint128 amount0Max, uint128 amount1Max) =
            _calculateInputs(amount0, delta, slippage);

        // (, int24 tick,,) = uniStateView.getSlot0(uniPoolKey.toId());
        // int24 tickSpacing = uniPoolKey.tickSpacing;
        // int24 tickLower = ((tick - tickSpacing) / tickSpacing) * tickSpacing;
        // int24 tickUpper = ((tick + tickSpacing) / tickSpacing) * tickSpacing;
        // uint128 amount0Max = 0.1 ether; // token0 side
        // uint128 amount1Max = 100e6; // token1 side
        // uint256 liquidity = 1e6;

        (uint256 used0, uint256 used1) =
            _univ4LiquidityMint(owner, tickLower, tickUpper, liquidity, uint128(amount0Max), uint128(amount1Max));
    }
}
