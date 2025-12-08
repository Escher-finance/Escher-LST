// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Ownable2Step, Ownable} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {IPositionManager} from "univ4-periphery/interfaces/IPositionManager.sol";
import {Actions} from "univ4-periphery/libraries/Actions.sol";
import {PoolKey} from "univ4-core/types/PoolKey.sol";
import {Currency, CurrencyLibrary} from "univ4-core/types/Currency.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

using CurrencyLibrary for Currency;
using SafeERC20 for IERC20;

contract IlSolver is Ownable2Step {
    IPositionManager public s_posm;
    PoolKey public s_poolKey;
    uint256 public s_positionTokenId;
    bool public s_ethLiquidityPosition;

    error IlSolver_wrongETHValueSent(uint256 needed, uint256 got);
    error IlSolver_wrongERC20Allowance(IERC20 token, uint256 needed, uint256 got);

    constructor(address _owner, IPositionManager _posm, PoolKey memory _poolKey) Ownable(_owner) {
        s_posm = _posm;
        s_poolKey = _poolKey;
        // Since it uses numerical sorting of addresses only the `currency0` can be ETH
        s_ethLiquidityPosition = _poolKey.currency0.isAddressZero();
    }

    modifier univ4AttachAndRefund(uint128 _amount0Max, uint128 _amount1Max) {
        uint256 amount0Max = uint256(_amount0Max);
        uint256 amount1Max = uint256(_amount1Max);
        PoolKey memory key = s_poolKey;
        address _this = address(this);
        address sender = msg.sender;

        uint256 b0Before = key.currency0.balanceOfSelf();
        uint256 b1Before = key.currency1.balanceOfSelf();

        if (s_ethLiquidityPosition) {
            if (msg.value < amount0Max) {
                revert IlSolver_wrongETHValueSent(amount0Max, msg.value);
            }
        } else {
            IERC20 t0 = IERC20(Currency.unwrap(key.currency0));
            t0.safeTransferFrom(sender, _this, amount0Max);
        }
        IERC20 t1 = IERC20(Currency.unwrap(key.currency1));
        t1.safeTransferFrom(sender, _this, amount1Max);

        _;

        uint256 b0After = key.currency0.balanceOfSelf();
        uint256 b1After = key.currency1.balanceOfSelf();

        uint256 used0 = b0Before + amount0Max - b0After;
        uint256 used1 = b1Before + amount1Max - b1After;
        require(used0 <= amount0Max);
        require(used1 <= amount1Max);
        uint256 refund0 = amount0Max - used0;
        uint256 refund1 = amount1Max - used1;
        if (refund0 > 0) {
            key.currency0.transfer(sender, refund0);
        }
        if (refund1 > 0) {
            key.currency1.transfer(sender, refund1);
        }
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
    ) public payable onlyOwner univ4AttachAndRefund(amount0Max, amount1Max) {
        if (s_positionTokenId == 0) {
            _univ4LiquidityMint(tickLower, tickUpper, liquidity, amount0Max, amount1Max);
        } else {
            _univ4LiquidityAdd(tickLower, tickUpper, liquidity, amount0Max, amount1Max);
        }
    }
}
