// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Actions} from "./libraries/Actions.sol";
import {IPositionManagerLike} from "./interfaces/PositionManagerLike.sol";
import {PMTypes} from "./interfaces/PositionManagerLike.sol";

/// A tiny helper that encodes actions and params per the Uniswap v4 Position Manager guide
/// and forwards them to a PositionManager-like contract.
contract PMRouter {
    IPositionManagerLike public immutable positionManager;

    constructor(IPositionManagerLike _pm) {
        positionManager = _pm;
    }

    /// Example: mint a position then settle the pair, per docs
    function mintPosition(
        PMTypes.PoolKey calldata poolKey,
        int24 tickLower,
        int24 tickUpper,
        uint128 liquidity,
        uint256 amount0Max,
        uint256 amount1Max,
        address recipient,
        uint256 deadline
    ) external payable {
        bytes memory actions = abi.encodePacked(
            Actions.MINT_POSITION,
            Actions.SETTLE_PAIR
        );

        bytes[] memory params = new bytes[](2);
        params[0] = abi.encode(
            poolKey,
            tickLower,
            tickUpper,
            liquidity,
            amount0Max,
            amount1Max,
            recipient,
            bytes("")
        );
        params[1] = abi.encode(poolKey.currency0, poolKey.currency1);

        positionManager.modifyLiquidities(abi.encode(actions, params), deadline);
    }

    /// Example: burn a position and take the pair
    function burnPosition(
        uint256 tokenId,
        uint256 amount0Min,
        uint256 amount1Min,
        address currency0,
        address currency1,
        address recipient,
        uint256 deadline
    ) external payable {
        bytes memory actions = abi.encodePacked(
            uint256(0x03), // BURN_POSITION not listed in our Actions lib; using 0x03 by convention here
            Actions.TAKE_PAIR
        );

        bytes[] memory params = new bytes[](2);
        params[0] = abi.encode(tokenId, amount0Min, amount1Min, bytes(""));
        params[1] = abi.encode(currency0, currency1, recipient);

        positionManager.modifyLiquidities(abi.encode(actions, params), deadline);
    }
}


