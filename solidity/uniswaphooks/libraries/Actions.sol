// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

/// Convenience constants matching the Uniswap v4 Position Manager guide.
/// These are provided for reference if you later wire a Position Manager based
/// flow to batch actions per docs: https://docs.uniswap.org/contracts/v4/guides/position-manager
library Actions {
    uint256 internal constant INCREASE_LIQUIDITY = 0x00;
    uint256 internal constant DECREASE_LIQUIDITY = 0x01;
    uint256 internal constant MINT_POSITION      = 0x02;
    uint256 internal constant BURN_6909          = 0x18;

    uint256 internal constant SETTLE_PAIR        = 0x0d;
    uint256 internal constant TAKE_PAIR          = 0x11;
    uint256 internal constant CLOSE_CURRENCY     = 0x12;
    uint256 internal constant CLEAR_OR_TAKE      = 0x13;
}


