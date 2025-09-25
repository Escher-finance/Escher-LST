// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import {LimitOrderHook} from "../LimitOrderHook.sol";
import {IPoolLike, ISwapper} from "../interfaces/PoolLike.sol";
import {SimpleSwapper} from "../ISimpleSwapper.sol";

contract MockPool is IPoolLike {
    uint160 public p;
    bool public lastZeroForOne;
    int256 public lastAmountSpecified;
    bytes public lastData;

    constructor(uint160 start) { p = start; }

    function setPrice(uint160 v) external { p = v; }

    function sqrtPriceX96() external view returns (uint160) { return p; }

    function swap(address, bool zeroForOne, int256 amountSpecified, bytes calldata data)
        external
        returns (int256 amount0, int256 amount1)
    {
        lastZeroForOne = zeroForOne;
        lastAmountSpecified = amountSpecified;
        lastData = data;
        // Return dummy values
        return (amountSpecified, -amountSpecified);
    }
}

contract LimitOrderHookTest is Test {
    LimitOrderHook hook;
    SimpleSwapper swapper;
    MockPool pool;

    function setUp() public {
        swapper = new SimpleSwapper();
        hook = new LimitOrderHook(ISwapper(swapper));
        pool = new MockPool(uint160(1 << 96)); // price=1.0 in Q64.96
    }

    function test_place_and_execute_zeroForOne_when_price_up() public {
        // zeroForOne true -> execute when current >= trigger
        uint160 trigger = uint160((uint256(11) * (1 << 96)) / 10); // 1.1
        uint256 id = hook.placeOrder(IPoolLike(pool), trigger, true, int256(1000), bytes("hi"));

        // not yet
        vm.expectRevert();
        hook.execute(id);

        // cross trigger
        pool.setPrice(trigger);
        hook.execute(id);
        // executed without revert
    }

    function test_place_and_execute_oneForZero_when_price_down() public {
        // zeroForOne false -> execute when current <= trigger
        uint160 trigger = uint160((uint256(9) * (1 << 96)) / 10); // 0.9
        uint256 id = hook.placeOrder(IPoolLike(pool), trigger, false, int256(500), bytes("hi"));

        // not yet
        vm.expectRevert();
        hook.execute(id);

        // cross trigger downward
        pool.setPrice(trigger);
        hook.execute(id);
    }

    function test_cancel() public {
        uint256 id = hook.placeOrder(IPoolLike(pool), uint160(1 << 96), true, int256(1), bytes(""));
        hook.cancelOrder(id);
        vm.expectRevert();
        hook.execute(id);
    }
}


