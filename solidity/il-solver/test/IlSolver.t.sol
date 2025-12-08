// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import {Test, console} from "forge-std/Test.sol";
import {IlSolver, IPositionManager, PoolKey, Currency, CurrencyLibrary, IERC20} from "../src/IlSolver.sol";
import {IHooks} from "univ4-core/interfaces/IHooks.sol";
import {IPoolManager} from "univ4-core/interfaces/IPoolManager.sol";
import {IStateView, PoolId} from "univ4-periphery/interfaces/IStateView.sol";
import {LiquidityAmounts} from "univ4-core/../test/utils/LiquidityAmounts.sol";
import {TickMath} from "univ4-core/libraries/TickMath.sol";

using CurrencyLibrary for Currency;

contract IlSolverTest is Test {
    IlSolver c;
    address owner;
    PoolKey key;
    PoolId id;
    IPoolManager poolManager;
    IStateView stateView;

    function setUp() public {
        vm.createSelectFork("mainnet", 23968000);
        owner = makeAddr("owner");

        // https://docs.uniswap.org/contracts/v4/deployments
        IPositionManager posm = IPositionManager(0xbD216513d74C8cf14cf4747E6AaA6420FF64ee9e);
        poolManager = IPoolManager(0x000000000004444c5dc75cB358380D2e3dE08A90);
        stateView = IStateView(0x7fFE42C4a5DEeA5b0feC41C94C136Cf115597227);

        key = PoolKey({
            currency0: CurrencyLibrary.ADDRESS_ZERO,
            currency1: Currency.wrap(0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48),
            fee: 3000,
            tickSpacing: 60,
            hooks: IHooks(address(0))
        });
        bytes32 rawId = bytes32(0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d);
        assertEq(keccak256(abi.encode(key)), rawId);
        id = PoolId.wrap(rawId);

        c = new IlSolver(owner, posm, key);
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

    function testUniV4Mint() public {
        assertEq(c.s_positionTokenId(), 0);
        uint256 input0 = 1 ether;
        int24 delta = 488; // 5% in ticks

        (uint160 sqrtPriceX96, int24 tick,,) = stateView.getSlot0(id);
        int24 tickSpacing = key.tickSpacing;
        int24 tickLower = ((tick - delta) / tickSpacing) * tickSpacing;
        int24 tickUpper = ((tick + delta) / tickSpacing) * tickSpacing;

        uint160 sqrtPriceAX96 = TickMath.getSqrtPriceAtTick(tickLower);
        uint160 sqrtPriceBX96 = TickMath.getSqrtPriceAtTick(tickUpper);
        uint128 liquidity = LiquidityAmounts.getLiquidityForAmount0(sqrtPriceAX96, sqrtPriceBX96, input0);

        (uint256 required0, uint256 required1) =
            LiquidityAmounts.getAmountsForLiquidity(sqrtPriceX96, sqrtPriceAX96, sqrtPriceBX96, liquidity);

        IERC20 t1 = IERC20(Currency.unwrap(key.currency1));
        t1.approve(address(c), required1);
        c.univ4LiquidityAdd{value: required0}(tickLower, tickUpper, liquidity, uint128(required0), uint128(required1));

        assertGt(c.s_positionTokenId(), 0);
    }
}
