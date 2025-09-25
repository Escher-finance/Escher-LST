// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {BaseHook} from "v4-periphery/src/utils/BaseHook.sol";
// Import IPoolManager from the same tree BaseHook uses to avoid type duplication
import {IPoolManager} from "v4-periphery/lib/v4-core/src/interfaces/IPoolManager.sol";

/// Minimal BaseHook extension placeholder; wire limit-order checks in relevant callbacks.
abstract contract LimitOrderBaseHook is BaseHook {
    struct Order {
        // who can execute this order via swap
        address owner;
        // direction: true = token0->token1, false = token1->token0
        bool zeroForOne;
        // trigger price in Q64.96
        uint160 triggerSqrtPriceX96;
        // signed amount (positive exact-in / negative exact-out)
        int256 amountSpecified;
        bool active;
    }

    // pool id hash => owner => order
    mapping(bytes32 => mapping(address => Order)) public orders;

    constructor(IPoolManager manager) BaseHook(manager) {}

    // lightweight admin to place/cancel; in production add auth/permit
    function placeOrder(
        address currency0,
        address currency1,
        uint24 fee,
        address hooks,
        bool zeroForOne,
        uint160 triggerSqrtPriceX96,
        int256 amountSpecified
    ) external {
        bytes32 id = _poolId(currency0, currency1, fee, hooks);
        orders[id][msg.sender] = Order({
            owner: msg.sender,
            zeroForOne: zeroForOne,
            triggerSqrtPriceX96: triggerSqrtPriceX96,
            amountSpecified: amountSpecified,
            active: true
        });
    }

    function cancelOrder(address currency0, address currency1, uint24 fee, address hooks) external {
        bytes32 id = _poolId(currency0, currency1, fee, hooks);
        Order storage o = orders[id][msg.sender];
        require(o.active && o.owner == msg.sender, "no order");
        o.active = false;
    }

    // Helper callable by an offchain keeper or router before initiating a swap.
    function validateAndConsume(
        address currency0,
        address currency1,
        uint24 fee,
        address hooks,
        address owner,
        int256 amountSpecified,
        uint160 observedSqrtPriceX96
    ) external {
        bytes32 id = _poolId(currency0, currency1, fee, hooks);
        Order storage o = orders[id][owner];
        require(o.active && o.owner != address(0), "no ord");
        require(amountSpecified == o.amountSpecified, "amount");
        bool priceOk = o.zeroForOne ? (observedSqrtPriceX96 >= o.triggerSqrtPriceX96)
                                    : (observedSqrtPriceX96 <= o.triggerSqrtPriceX96);
        require(priceOk, "price");
        o.active = false;
    }

    function _poolId(address currency0, address currency1, uint24 fee, address hooks) internal pure returns (bytes32) {
        return keccak256(abi.encode(currency0, currency1, fee, hooks));
    }
}


