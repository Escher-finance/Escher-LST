// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import {Test, console} from "forge-std/Test.sol";
import {
    IlSolver,
    IPositionManager,
    PoolKey,
    Currency,
    CurrencyLibrary,
    IERC20,
    IL2Pool,
    L2Encoder
} from "../src/IlSolver.sol";
import {IHooks} from "univ4-core/interfaces/IHooks.sol";
import {IPoolManager} from "univ4-core/interfaces/IPoolManager.sol";
import {IStateView, PoolId} from "univ4-periphery/interfaces/IStateView.sol";
import {LiquidityAmounts} from "univ4-core/../test/utils/LiquidityAmounts.sol";
import {TickMath} from "univ4-core/libraries/TickMath.sol";
import {PositionInfo, PositionInfoLibrary} from "univ4-periphery/libraries/PositionInfoLibrary.sol";
import {Position} from "univ4-core/libraries/Position.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

using CurrencyLibrary for Currency;
using SafeERC20 for IERC20;
using PositionInfoLibrary for PositionInfo;

contract IlSolverTest is Test {
    IlSolver c;
    address owner;
    PoolKey key;
    PoolId id;
    IPositionManager posm;
    IPoolManager poolManager;
    IStateView stateView;
    IL2Pool l2Pool;
    L2Encoder l2Encoder;
    IERC20 l2Underlying;

    function setUp() public {
        vm.createSelectFork("base", 39260000);
        owner = makeAddr("owner");

        address usdc = 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913;

        // https://docs.uniswap.org/contracts/v4/deployments
        posm = IPositionManager(0x7C5f5A4bBd8fD63184577525326123B519429bDc);
        poolManager = IPoolManager(address(posm.poolManager()));
        stateView = IStateView(0xA3c0c9b65baD0b08107Aa264b0f3dB444b867A71);
        l2Pool = IL2Pool(0xA238Dd80C259a72e81d7e4664a9801593F98d1c5);
        l2Encoder = L2Encoder(0x39e97c588B2907Fb67F44fea256Ae3BA064207C5);
        l2Underlying = IERC20(usdc);

        key = PoolKey({
            currency0: CurrencyLibrary.ADDRESS_ZERO,
            currency1: Currency.wrap(usdc),
            fee: 500,
            tickSpacing: 10,
            hooks: IHooks(address(0))
        });
        bytes32 rawId = bytes32(0x96d4b53a38337a5733179751781178a2613306063c511b78cd02684739288c0a);
        assertEq(keccak256(abi.encode(key)), rawId);
        id = PoolId.wrap(rawId);

        c = new IlSolver(owner, posm, key, l2Pool, l2Encoder, l2Underlying);
        assertEq(c.owner(), owner);
        assertEq(address(c.s_posm()), address(posm));

        deal(owner, 1 ether);
        deal(Currency.unwrap(key.currency1), owner, 10000e6);

        assertGt(key.currency0.balanceOf(owner), 0);
        assertGt(key.currency1.balanceOf(owner), 0);
        uint128 liquidity = stateView.getLiquidity(id);
        assertGt(liquidity, 0);

        vm.startPrank(owner);
    }

    function _calculateInputs(uint256 amount0, int24 delta, uint256 slippage)
        private
        returns (int24 tickLower, int24 tickUpper, uint128 liquidity, uint128 amount0Max, uint128 amount1Max)
    {
        (uint160 sqrtPriceX96, int24 tick,,) = stateView.getSlot0(id);
        int24 tickSpacing = key.tickSpacing;
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

    function _mintUniV4Pos(uint256 amount0, int24 delta, uint256 slippage) private {
        (int24 tickLower, int24 tickUpper, uint128 liquidity, uint128 amount0Max, uint128 amount1Max) =
            _calculateInputs(amount0, delta, slippage);

        IERC20 t1 = IERC20(Currency.unwrap(key.currency1));
        t1.approve(address(c), amount1Max);
        c.univ4LiquidityAdd{value: amount0Max}(
            tickLower, tickUpper, liquidity, uint128(amount0Max), uint128(amount1Max)
        );

        assertGt(posm.getPositionLiquidity(c.s_positionTokenId()), 0);
    }

    function testUniV4Mint() public {
        assertEq(c.s_positionTokenId(), 0);
        int24 delta = 488; // 5% in ticks
        uint256 slippage = 10; // 10%
        _mintUniV4Pos(1 ether, delta, slippage);
        assertGt(c.s_positionTokenId(), 0);
    }

    function testUniV4Increase() public {
        int24 delta = 488; // 5% in ticks
        uint256 slippage = 10; // 10%
        _mintUniV4Pos(1 ether, delta, slippage);

        uint128 oldLiquidity = posm.getPositionLiquidity(c.s_positionTokenId());

        (,, uint128 liquidity, uint128 amount0Max, uint128 amount1Max) = _calculateInputs(0.5 ether, delta, slippage);
        IERC20 t1 = IERC20(Currency.unwrap(key.currency1));
        t1.approve(address(c), amount1Max);
        c.univ4LiquidityAdd{value: amount0Max}(0, 0, liquidity, amount0Max, amount1Max);

        uint128 newLiquidity = posm.getPositionLiquidity(c.s_positionTokenId());
        assertGt(newLiquidity, oldLiquidity);
    }
}
