// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Ownable2Step, Ownable} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {IPositionManager} from "univ4-periphery/interfaces/IPositionManager.sol";
import {Actions} from "univ4-periphery/libraries/Actions.sol";
import {PoolKey} from "univ4-core/types/PoolKey.sol";
import {Currency, CurrencyLibrary} from "univ4-core/types/Currency.sol";

using CurrencyLibrary for Currency;

contract IlSolver is Ownable2Step {
    IPositionManager public s_posm;
    PoolKey public s_poolKey;
    uint256 public s_positionTokenId;
    bool public s_ethLiquidityPosition;

    constructor(address _owner, IPositionManager _posm, PoolKey memory _poolKey) Ownable(_owner) {
        s_posm = _posm;
        s_poolKey = _poolKey;
        // Since it uses numerical sorting of addresses only the `currency0` can be ETH
        s_ethLiquidityPosition = _poolKey.currency0.isAddressZero();
    }

    function _univ4LiquidityMint(
        int24 tickLower,
        int24 tickUpper,
        uint256 liquidity,
        uint128 amount0Max,
        uint128 amount1Max
    ) private {
        bool ethLiquidityPosition = s_ethLiquidityPosition;
        bytes memory actions;
        address _this = address(this);
        if (ethLiquidityPosition) {
            actions = abi.encodePacked(uint8(Actions.MINT_POSITION), uint8(Actions.SETTLE_PAIR), uint8(Actions.SWEEP));
        } else {
            actions = abi.encodePacked(uint8(Actions.MINT_POSITION), uint8(Actions.SETTLE_PAIR));
        }
        PoolKey memory _key = s_poolKey;
        bytes memory params0 =
            abi.encode(_key, tickLower, tickUpper, liquidity, amount0Max, amount1Max, _this, bytes(""));
        bytes memory params1 = abi.encode(_key.currency0, _key.currency1);
        bytes[] memory params;
        if (ethLiquidityPosition) {
            params = new bytes[](3);
            params[0] = params0;
            params[1] = params1;
            params[2] = abi.encode(CurrencyLibrary.ADDRESS_ZERO, _this);
        } else {
            params = new bytes[](2);
            params[0] = params0;
            params[1] = params1;
        }
        uint256 deadline = block.timestamp;
        uint256 positionId = s_posm.nextTokenId();
        s_posm.modifyLiquidities{value: ethLiquidityPosition ? amount0Max : 0}(abi.encode(actions, params), deadline);
        s_positionTokenId = positionId;
    }

    function _univ4LiquidityAdd(
        int24 tickLower,
        int24 tickUpper,
        uint256 liquidity,
        uint128 amount0Max,
        uint128 amount1Max
    ) private {
        bool ethLiquidityPosition = s_ethLiquidityPosition;
        bytes memory actions;
        address _this = address(this);
        if (ethLiquidityPosition) {
            actions =
                abi.encodePacked(uint8(Actions.INCREASE_LIQUIDITY), uint8(Actions.SETTLE_PAIR), uint8(Actions.SWEEP));
        } else {
            actions = abi.encodePacked(uint8(Actions.INCREASE_LIQUIDITY), uint8(Actions.SETTLE_PAIR));
        }
        uint256 _tokenId = s_positionTokenId;
        PoolKey memory _key = s_poolKey;
        bytes memory params0 =
            abi.encode(_tokenId, tickLower, tickUpper, liquidity, amount0Max, amount1Max, _this, bytes(""));
        bytes memory params1 = abi.encode(_key.currency0, _key.currency1);
        bytes[] memory params;
        if (ethLiquidityPosition) {
            params = new bytes[](3);
            params[0] = params0;
            params[1] = params1;
            params[2] = abi.encode(CurrencyLibrary.ADDRESS_ZERO, _this);
        } else {
            params = new bytes[](2);
            params[0] = params0;
            params[1] = params1;
        }
        uint256 deadline = block.timestamp;
        s_posm.modifyLiquidities{value: ethLiquidityPosition ? amount0Max : 0}(abi.encode(actions, params), deadline);
    }

    function univ4LiquidityAdd(
        int24 tickLower,
        int24 tickUpper,
        uint256 liquidity,
        uint128 amount0Max,
        uint128 amount1Max
    ) public onlyOwner {
        if (s_positionTokenId == 0) {
            _univ4LiquidityMint(tickLower, tickUpper, liquidity, amount0Max, amount1Max);
        } else {
            _univ4LiquidityAdd(tickLower, tickUpper, liquidity, amount0Max, amount1Max);
        }
    }
}
