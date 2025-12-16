// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Ownable2Step, Ownable} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {IPositionManager as IPositionManagerOriginal} from "univ4-periphery/interfaces/IPositionManager.sol";
import {Actions} from "univ4-periphery/libraries/Actions.sol";
import {PoolKey} from "univ4-core/types/PoolKey.sol";
import {Currency, CurrencyLibrary} from "univ4-core/types/Currency.sol";
import {IImmutableState} from "univ4-periphery/interfaces/IImmutableState.sol";
import {IL2Pool as IL2PoolOriginal} from "aavev3/interfaces/IL2Pool.sol";
import {IPool} from "aavev3/interfaces/IPool.sol";
import {L2Encoder} from "aavev3/helpers/L2Encoder.sol";
import {DataTypes} from "aavev3/protocol/libraries/types/DataTypes.sol";
import {ReserveConfiguration} from "aavev3/protocol/libraries/configuration/ReserveConfiguration.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

using CurrencyLibrary for Currency;
using SafeERC20 for IERC20;
using ReserveConfiguration for DataTypes.ReserveConfigurationMap;

interface IPositionManager is IPositionManagerOriginal {
    function permit2() external view returns (address);
}

interface IL2Pool is IL2PoolOriginal, IPool {}

interface IPermit2 {
    function allowance(address user, address token, address spender)
        external
        view
        returns (uint160 amount, uint48 expiration, uint48 nonce);
    function approve(address token, address spender, uint160 amount, uint48 expiration) external;
}

contract IlSolver is Ownable2Step {
    // Uniswap V4
    IPositionManager public s_posm;
    PoolKey public s_poolKey;
    uint256 public s_positionTokenId;
    bool public s_ethLiquidityPosition;

    // Aave V3
    IL2Pool s_l2Pool;
    L2Encoder s_l2Encoder;
    // Supplied token
    IERC20 s_l2Underlying;
    // Borrow token
    address s_l2Borrow;

    error IlSolver_wrongETHValueSent(uint256 needed, uint256 got);
    error IlSolver_wrongERC20Allowance(IERC20 token, uint256 needed, uint256 got);

    constructor(
        address _owner,
        IPositionManager _posm,
        PoolKey memory _poolKey,
        IL2Pool _l2Pool,
        L2Encoder _l2Encoder,
        IERC20 _l2Underlying,
        address _l2Borrow
    ) Ownable(_owner) {
        _posm.permit2();
        _posm.poolManager();
        s_posm = _posm;
        s_poolKey = _poolKey;
        // Since it uses numerical sorting of addresses only the `currency0` can be ETH
        s_ethLiquidityPosition = _poolKey.currency0.isAddressZero();

        s_l2Pool = _l2Pool;
        s_l2Encoder = _l2Encoder;
        s_l2Underlying = _l2Underlying;
        s_l2Borrow = _l2Borrow;

        s_l2Pool.setUserUseReserveAsCollateral(l2Pool.getReserveAToken(address(_l2Underlying)), true);
    }

    receive() external payable {}

    modifier univ4AttachAndRefund(uint128 _amount0Max, uint128 _amount1Max) {
        uint256 amount0Max = uint256(_amount0Max);
        uint256 amount1Max = uint256(_amount1Max);
        PoolKey memory key = s_poolKey;
        address _this = address(this);
        address sender = msg.sender;
        address _permit2 = s_posm.permit2();
        IPermit2 permit2 = IPermit2(_permit2);
        address _posm = address(s_posm);

        uint256 b0Before = key.currency0.balanceOfSelf();
        uint256 b1Before = key.currency1.balanceOfSelf();

        if (s_ethLiquidityPosition) {
            if (msg.value < amount0Max) {
                revert IlSolver_wrongETHValueSent(amount0Max, msg.value);
            }
        } else {
            IERC20 t0 = IERC20(Currency.unwrap(key.currency0));
            t0.safeTransferFrom(sender, _this, amount0Max);

            // permit2
            if (t0.allowance(_this, _permit2) < amount0Max) {
                t0.approve(_permit2, type(uint128).max);
            }
            (uint160 p2Allowance0,,) = permit2.allowance(_this, address(t0), _posm);
            if (p2Allowance0 < amount0Max) {
                permit2.approve(address(t0), _posm, type(uint128).max, type(uint48).max);
            }
        }
        IERC20 t1 = IERC20(Currency.unwrap(key.currency1));
        t1.safeTransferFrom(sender, _this, amount1Max);
        if (t1.allowance(_this, _permit2) < amount1Max) {
            t1.approve(_permit2, type(uint128).max);
        }
        (uint160 p2Allowance1,,) = permit2.allowance(_this, address(t1), _posm);
        if (p2Allowance1 < amount1Max) {
            permit2.approve(address(t1), _posm, type(uint128).max, type(uint48).max);
        }

        _;

        uint256 b0After = key.currency0.balanceOfSelf();
        uint256 b1After = key.currency1.balanceOfSelf();

        uint256 used0 = b0Before + amount0Max - b0After;
        uint256 used1 = b1Before + amount1Max - b1After;
        uint256 refund0 = used0 < amount0Max ? amount0Max - used0 : 0;
        uint256 refund1 = used1 < amount1Max ? amount1Max - used1 : 0;
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
        address posm = address(s_posm);
        if (!ethLiquidityPosition) {
            IERC20(Currency.unwrap(_key.currency0)).approve(posm, amount0Max);
        }
        IERC20(Currency.unwrap(_key.currency1)).approve(posm, amount1Max);
        s_posm.modifyLiquidities{value: ethLiquidityPosition ? amount0Max : 0}(abi.encode(actions, params), deadline);
        s_positionTokenId = positionId;
    }

    function _univ4LiquidityAdd(uint256 liquidity, uint128 amount0Max, uint128 amount1Max) private {
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
        bytes memory params0 = abi.encode(_tokenId, liquidity, amount0Max, amount1Max, bytes(""));
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
        address posm = address(s_posm);
        if (!ethLiquidityPosition) {
            IERC20(Currency.unwrap(_key.currency0)).approve(posm, amount0Max);
        }
        IERC20(Currency.unwrap(_key.currency1)).approve(posm, amount1Max);
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
            _univ4LiquidityAdd(liquidity, amount0Max, amount1Max);
        }
    }

    function aavev3Supply(uint256 amount) public onlyOwner {
        s_l2Underlying.safeTransferFrom(msg.sender, address(this), amount);
        if (s_l2Underlying.allowance(address(this), address(s_l2Pool)) < amount) {
            s_l2Underlying.approve(address(s_l2Pool), type(uint128).max);
        }
        bytes32 params = s_l2Encoder.encodeSupplyParams(address(s_l2Underlying), amount, 0);
        s_l2Pool.supply(params);
    }

    function aavev3Ltv() public view returns (uint256 ltv) {
        DataTypes.ReserveConfigurationMap memory map = s_l2Pool.getConfiguration(address(s_l2Underlying));
        ltv = map.getLtv();
    }

    function aavev3Borrow(uint256 amount) public onlyOwner {
        s_l2Pool.borrow(s_l2Borrow, amount, 2, 0, address(this));
    }
}
