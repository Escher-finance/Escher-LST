// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {IPoolLike, ISwapper} from "./interfaces/PoolLike.sol";

/// A minimal keeper-triggered limit order helper intended to be adapted into a v4 Hook.
/// Not a real v4 hook yet. Keeps state of orders and lets a keeper execute when price meets condition.
contract LimitOrderHook {
    struct Order {
        address owner;
        IPoolLike pool;
        // Target sqrt price in Q64.96 to trigger the order.
        uint160 triggerSqrtPriceX96;
        // Direction: true = token0->token1 (zeroForOne), false = token1->token0.
        bool zeroForOne;
        // Signed amount for swap; positive for exact-in, negative for exact-out.
        int256 amountSpecified;
        // Arbitrary payload forwarded to swapper/pool callback
        bytes data;
        // Whether the order is active
        bool active;
    }

    event OrderPlaced(
        uint256 indexed id,
        address indexed owner,
        address pool,
        uint160 triggerSqrtPriceX96,
        bool zeroForOne,
        int256 amountSpecified
    );
    event OrderCancelled(uint256 indexed id, address indexed owner);
    event OrderExecuted(uint256 indexed id, address indexed executor, int256 amount0, int256 amount1);

    ISwapper public immutable swapper;
    address public owner;

    uint256 private nextOrderId;
    mapping(uint256 => Order) public orders;

    modifier onlyOwner() {
        require(msg.sender == owner, "not owner");
        _;
    }

    constructor(ISwapper _swapper) {
        owner = msg.sender;
        swapper = _swapper;
        nextOrderId = 1;
    }

    function transferOwnership(address newOwner) external onlyOwner {
        owner = newOwner;
    }

    /// Place a new limit order.
    function placeOrder(
        IPoolLike pool,
        uint160 triggerSqrtPriceX96,
        bool zeroForOne,
        int256 amountSpecified,
        bytes calldata data
    ) external returns (uint256 id) {
        id = nextOrderId++;
        orders[id] = Order({
            owner: msg.sender,
            pool: pool,
            triggerSqrtPriceX96: triggerSqrtPriceX96,
            zeroForOne: zeroForOne,
            amountSpecified: amountSpecified,
            data: data,
            active: true
        });
        emit OrderPlaced(id, msg.sender, address(pool), triggerSqrtPriceX96, zeroForOne, amountSpecified);
    }

    /// Cancel an active order.
    function cancelOrder(uint256 id) external {
        Order storage o = orders[id];
        require(o.active, "inactive");
        require(o.owner == msg.sender, "not owner");
        o.active = false;
        emit OrderCancelled(id, msg.sender);
    }

    /// Execute an order if the pool price crossed the trigger.
    /// Anyone can call; useful for offchain keeper/automation.
    function execute(uint256 id) external {
        Order storage o = orders[id];
        require(o.active, "inactive");
        uint160 current = o.pool.sqrtPriceX96();

        // For buy-side triggers, you typically want: if zeroForOne (sell token0 for token1),
        // execute when price is at or above trigger; for the inverse, execute when at or below.
        // Exact strategy can be customized; here we pick a simple rule:
        bool shouldExecute = o.zeroForOne ? current >= o.triggerSqrtPriceX96 : current <= o.triggerSqrtPriceX96;
        require(shouldExecute, "price not met");

        o.active = false; // prevent reentry/reuse

        (int256 a0, int256 a1) = swapper.executeSwap(o.pool, o.zeroForOne, o.amountSpecified, o.data);
        emit OrderExecuted(id, msg.sender, a0, a1);
    }
}

