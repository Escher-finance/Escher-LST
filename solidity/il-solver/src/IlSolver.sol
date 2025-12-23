// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Ownable2Step, Ownable} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {IPositionManager as IPositionManagerOriginal} from "univ4-periphery/interfaces/IPositionManager.sol";
import {Actions} from "univ4-periphery/libraries/Actions.sol";
import {PoolKey} from "univ4-core/types/PoolKey.sol";
import {Currency, CurrencyLibrary} from "univ4-core/types/Currency.sol";
import {IImmutableState} from "univ4-periphery/interfaces/IImmutableState.sol";
import {IL2Pool as IL2PoolOriginal} from "aavev3/interfaces/IL2Pool.sol";
import {IAaveOracle} from "aavev3/interfaces/IAaveOracle.sol";
import {IPool} from "aavev3/interfaces/IPool.sol";
import {L2Encoder} from "aavev3/helpers/L2Encoder.sol";
import {DataTypes} from "aavev3/protocol/libraries/types/DataTypes.sol";
import {ReserveConfiguration} from "aavev3/protocol/libraries/configuration/ReserveConfiguration.sol";
import {IPoolDataProvider} from "aavev3/interfaces/IPoolDataProvider.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IlSolverMath} from "./core/EscherMath.sol";
import {IWETH} from "@common/IWETH.sol";

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
    IPositionManager public uniPosm;
    PoolKey public uniPoolKey;
    uint256 public uniPositionTokenId;
    // bool public s_ethLiquidityPosition;

    IWETH public immutable WETH;
    IERC20 public immutable collateral;

    // Aave V3
    IL2Pool public aavePool;
    L2Encoder public aaveEncoder;
    IPoolDataProvider public aaveDataProvider;
    // // Supplied token
    // IERC20 collateral;
    // // Borrow token
    // IERC20 s_l2Borrow;
    // Whether collateral has been set for the supplied token
    bool public aaveCollateralSet;

    IAaveOracle aaveOracle;

    error IlSolver_wrongETHValueSent(uint256 needed, uint256 got);
    error IlSolver_wrongERC20Allowance(IERC20 token, uint256 needed, uint256 got);

    constructor(
        address _owner,
        IWETH _weth,
        IERC20 _collateral,
        IPositionManager _uniPosm,
        PoolKey memory _uniPoolKey,
        IL2Pool _aavePool,
        L2Encoder _aaveEncoder,
        IPoolDataProvider _aaveDataProvider,
        IAaveOracle _aaveOracle
    ) Ownable(_owner) {
        WETH = _weth;

        _uniPosm.permit2();
        _uniPosm.poolManager();
        uniPosm = _uniPosm;

        require(_uniPoolKey.currency0.isAddressZero());
        require(Currency.unwrap(_uniPoolKey.currency1) == address(_collateral));

        uniPoolKey = _uniPoolKey;

        (,,,,, bool usageAsCollateralEnabled,,,,) = _aaveDataProvider.getReserveConfigurationData(_collateral);
        require(usageAsCollateralEnabled);
        aaveDataProvider = _aaveDataProvider;
        collateral = _collateral;
        aavePool = _aavePool;
        aaveEncoder = _aaveEncoder;
        aaveOracle = _aaveEncoder;
    }

    receive() external payable {}

    /// @dev Sets allowances and validates contract's funds to use in Uniswap V4
    /// @notice In the case there's not enough ETH (`_amount0Max`), it will attempt to unwrap the right amount of WETH
    modifier univ4AttachFunds(uint128 _amount0Max, uint128 _amount1Max) {
        uint256 amount0Max = uint256(_amount0Max);
        uint256 amount1Max = uint256(_amount1Max);
        PoolKey memory key = uniPoolKey;
        address _this = address(this);
        address _permit2 = uniPosm.permit2();
        IPermit2 permit2 = IPermit2(_permit2);
        address _posm = address(uniPosm);

        // Handle ETH

        uint256 ethBalance = _this.balance;
        uint256 ethNeeded = (ethBalance < amount0Max) ? amount0Max - ethBalance : 0;
        if (ethNeeded > 0) {
            WETH.withdraw(ethNeeded);
        }

        // Handle collateral

        IERC20 t1 = IERC20(Currency.unwrap(key.currency1));
        if (t1.allowance(_this, _permit2) < amount1Max) {
            t1.approve(_permit2, type(uint128).max);
        }
        (uint160 p2Allowance1,,) = permit2.allowance(_this, address(t1), _posm);
        if (p2Allowance1 < amount1Max) {
            permit2.approve(address(t1), _posm, type(uint128).max, type(uint48).max);
        }

        _;
    }

    function _univ4LiquidityMint(
        int24 tickLower,
        int24 tickUpper,
        uint256 liquidity,
        uint128 amount0Max,
        uint128 amount1Max
    ) private returns (uint256 used0, uint256 used1) {
        address _this = address(this);
        PoolKey memory _key = uniPoolKey;

        bytes memory actions =
            abi.encodePacked(uint8(Actions.MINT_POSITION), uint8(Actions.SETTLE_PAIR), uint8(Actions.SWEEP));

        bytes[] memory params = new bytes[](3);
        params[0] = abi.encode(_key, tickLower, tickUpper, liquidity, amount0Max, amount1Max, _this, bytes(""));
        params[1] = abi.encode(_key.currency0, _key.currency1);
        params[2] = abi.encode(CurrencyLibrary.ADDRESS_ZERO, _this);

        uint256 deadline = block.timestamp;
        uint256 positionId = uniPosm.nextTokenId();
        address posm = address(uniPosm);
        IERC20(Currency.unwrap(_key.currency1)).approve(posm, amount1Max);

        uint256 b0Before = _key.currency0.balanceOfSelf();
        uint256 b1Before = _key.currency1.balanceOfSelf();

        uniPosm.modifyLiquidities{value: amount0Max}(abi.encode(actions, params), deadline);

        uniPositionTokenId = positionId;

        uint256 b0After = _key.currency0.balanceOfSelf();
        uint256 b1After = _key.currency1.balanceOfSelf();

        used0 = b0Before + amount0Max - b0After;
        used1 = b1Before + amount1Max - b1After;
    }

    function _univ4LiquidityIncrement(uint256 liquidity, uint128 amount0Max, uint128 amount1Max)
        private
        returns (uint256 used0, uint256 used1)
    {
        address _this = address(this);
        uint256 _tokenId = uniPositionTokenId;
        PoolKey memory _key = uniPoolKey;

        bytes memory actions =
            abi.encodePacked(uint8(Actions.INCREASE_LIQUIDITY), uint8(Actions.SETTLE_PAIR), uint8(Actions.SWEEP));

        bytes[] memory params = new bytes[](3);
        params[0] = abi.encode(_tokenId, liquidity, amount0Max, amount1Max, bytes(""));
        params[1] = abi.encode(_key.currency0, _key.currency1);
        params[2] = abi.encode(CurrencyLibrary.ADDRESS_ZERO, _this);

        uint256 deadline = block.timestamp;
        address posm = address(uniPosm);
        IERC20(Currency.unwrap(_key.currency1)).approve(posm, amount1Max);

        uint256 b0Before = _key.currency0.balanceOfSelf();
        uint256 b1Before = _key.currency1.balanceOfSelf();

        uniPosm.modifyLiquidities{value: amount0Max}(abi.encode(actions, params), deadline);

        uint256 b0After = _key.currency0.balanceOfSelf();
        uint256 b1After = _key.currency1.balanceOfSelf();

        used0 = b0Before + amount0Max - b0After;
        used1 = b1Before + amount1Max - b1After;
    }

    function _univ4LiquidityAdd(
        int24 tickLower,
        int24 tickUpper,
        uint256 liquidity,
        uint128 amount0Max,
        uint128 amount1Max
    ) private univ4AttachFunds(amount0Max, amount1Max) {
        if (uniPositionTokenId == 0) {
            _univ4LiquidityMint(tickLower, tickUpper, liquidity, amount0Max, amount1Max);
        } else {
            _univ4LiquidityIncrement(liquidity, amount0Max, amount1Max);
        }
    }

    function _aavev3Supply(uint256 amount) private {
        collateral.safeTransferFrom(msg.sender, address(this), amount);
        if (collateral.allowance(address(this), address(s_l2Pool)) < amount) {
            collateral.approve(address(s_l2Pool), type(uint128).max);
        }
        bytes32 params = aaveEncoder.encodeSupplyParams(address(collateral), amount, 0);
        aavePool.supply(params);

        if (!aaveCollateralSet) {
            aavePool.setUserUseReserveAsCollateral(address(collateral), true);
            aaveCollateralSet = true;
        }
    }

    function _aavev3Borrow(uint256 amount) private {
        s_l2Pool.borrow(address(s_l2Borrow), amount, 2, 0, address(this));
    }

    function aavev3Ltv() public view returns (uint256 ltv) {
        DataTypes.ReserveConfigurationMap memory map = s_l2Pool.getConfiguration(address(collateral));
        ltv = map.getLtv();
    }

    function oraclePrice(address asset) public returns (uint256 price) {
        price = s_oracle.getAssetPrice(asset);
    }

    function openHedgedPosition(
        uint128 amount0,
        uint128 amount1,
        uint128 amount0Max,
        uint128 amount1Max,
        int24 tickLower,
        int24 tickUpper,
        uint256 liquidity
    ) public onlyOwner {
        // FIXME make sure internal functions are using contracts balance and not caller's allowance
        // TODO allowances + wrap eth

        (uint256 used0,) = _univ4LiquidityMint(tickLower, tickUpper, liquidity, amount0Max, amount1Max);

        uint256 borrowedAmountNeeded = used0;

        uint256 ltv = aavev3Ltv();
        uint256 priceUsd = oraclePrice(address(s_l2Borrow));

        uint256 collateralNeeded = IlSolverMath.calculateCollateralAmount(borrowedAmountNeeded, priceUsd, ltv);

        aavev3Supply(collateralNeeded);

        (uint256 iterations, bool isEnough, uint256 totalBorrowed,) =
            IlSolverMath.hedgingLoop(collateralNeeded, borrowedAmountNeeded, priceUsd, ltv);

        require(isEnough, "insufficient collateral");

        uint256 borrowPerIter = totalBorrowed / iterations;
        for (uint256 i = 0; i < iterations; i++) {
            _aavev3Borrow(borrowPerIter);
        }
    }
}
