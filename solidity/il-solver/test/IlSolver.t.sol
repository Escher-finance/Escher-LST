// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import {Test, console} from "forge-std/Test.sol";
import {
    IlSolver,
    IPositionManager,
    PoolKey,
    Currency,
    CurrencyLibrary,
    IERC20,
    IL2Pool,
    L2Encoder,
    IAaveOracle,
    IPoolDataProvider,
    IWETH,
    IUniversalRouter
} from "../src/IlSolver.sol";
import {IHooks} from "univ4-core/interfaces/IHooks.sol";
import {IPoolManager} from "univ4-core/interfaces/IPoolManager.sol";
import {IStateView, PoolId} from "univ4-periphery/interfaces/IStateView.sol";
import {LiquidityAmounts} from "univ4-core/../test/utils/LiquidityAmounts.sol";
import {TickMath} from "univ4-core/libraries/TickMath.sol";
import {PositionInfo, PositionInfoLibrary} from "univ4-periphery/libraries/PositionInfoLibrary.sol";
import {Position} from "univ4-core/libraries/Position.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

using CurrencyLibrary for Currency;
using SafeERC20 for IERC20;
using PositionInfoLibrary for PositionInfo;

contract IlSolverTest is Test {
    address usdc = 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913;
    address weth = 0x4200000000000000000000000000000000000006;

    IWETH WETH;
    IERC20 collateral;
    IERC20 reserve;

    IlSolver c;
    address owner;

    PoolKey key;
    PoolId id;
    IPositionManager uniPosm;
    IPoolManager uniPoolManager;
    IStateView uniStateView;
    IUniversalRouter uniRouter;

    IL2Pool aavePool;
    L2Encoder aaveEncoder;
    IPoolDataProvider aaveDataProvider;
    IAaveOracle aaveOracle;

    function setUp() public {
        vm.createSelectFork("base", 39260000);
        owner = makeAddr("owner");

        // https://docs.uniswap.org/contracts/v4/deployments
        uniPosm = IPositionManager(0x7C5f5A4bBd8fD63184577525326123B519429bDc);
        uniPoolManager = IPoolManager(address(uniPosm.poolManager()));
        uniStateView = IStateView(0xA3c0c9b65baD0b08107Aa264b0f3dB444b867A71);
        aavePool = IL2Pool(0xA238Dd80C259a72e81d7e4664a9801593F98d1c5);
        aaveEncoder = L2Encoder(0x39e97c588B2907Fb67F44fea256Ae3BA064207C5);
        aaveOracle = IAaveOracle(0x2Cc0Fc26eD4563A5ce5e8bdcfe1A2878676Ae156);
        aaveDataProvider = IPoolDataProvider(0x0F43731EB8d45A581f4a36DD74F5f358bc90C73A);
        uniRouter = IUniversalRouter(0x6fF5693b99212Da76ad316178A184AB56D299b43);

        WETH = IWETH(weth);
        collateral = IERC20(usdc);
        reserve = IERC20(aavePool.getReserveAToken(usdc));

        key = PoolKey({
            currency0: CurrencyLibrary.ADDRESS_ZERO,
            currency1: Currency.wrap(usdc),
            fee: 500,
            tickSpacing: 10,
            hooks: IHooks(address(0))
        });

        bytes32 rawId = bytes32(0x96d4b53a38337a5733179751781178a2613306063c511b78cd02684739288c0a);
        assertEq(keccak256(abi.encode(key)), rawId);
        id = PoolId.wrap(rawId);

        c = new IlSolver(
            owner, WETH, collateral, uniPosm, key, uniRouter, aavePool, aaveEncoder, aaveDataProvider, aaveOracle
        );
        assertEq(c.owner(), owner);
        assertEq(address(c.uniPosm()), address(uniPosm));

        // fund contract
        deal(address(c), 1 ether);
        deal(Currency.unwrap(key.currency1), address(c), 10000e6);

        assertGt(key.currency0.balanceOf(address(c)), 0);
        assertGt(key.currency1.balanceOf(address(c)), 0);
        uint128 liquidity = uniStateView.getLiquidity(id);
        assertGt(liquidity, 0);

        vm.startPrank(owner);
    }

    function _calculateInputs(uint256 amount0, int24 delta, uint256 slippage)
        private
        view
        returns (int24 tickLower, int24 tickUpper, uint128 liquidity, uint128 amount0Max, uint128 amount1Max)
    {
        (uint160 sqrtPriceX96, int24 tick,,) = uniStateView.getSlot0(id);
        int24 tickSpacing = key.tickSpacing;
        tickLower = ((tick - delta) / tickSpacing) * tickSpacing;
        tickUpper = ((tick + delta) / tickSpacing) * tickSpacing;

        uint160 sqrtPriceAX96 = TickMath.getSqrtPriceAtTick(tickLower);
        uint160 sqrtPriceBX96 = TickMath.getSqrtPriceAtTick(tickUpper);
        liquidity = LiquidityAmounts.getLiquidityForAmount0(sqrtPriceAX96, sqrtPriceBX96, amount0);

        (uint256 required0, uint256 required1) =
            LiquidityAmounts.getAmountsForLiquidity(sqrtPriceX96, sqrtPriceAX96, sqrtPriceBX96, liquidity);

        amount0Max = uint128(required0 * (100 + slippage) / 100);
        amount1Max = uint128(required1 * (100 + slippage) / 100);
    }

    function _mintUniV4Pos(uint256 amount0, int24 delta, uint256 slippage) private {
        (int24 tickLower, int24 tickUpper, uint128 liquidity, uint128 amount0Max, uint128 amount1Max) =
            _calculateInputs(amount0, delta, slippage);
        c.univ4LiquidityAdd(tickLower, tickUpper, liquidity, uint128(amount0Max), uint128(amount1Max));
        assertGt(uniPosm.getPositionLiquidity(c.uniPositionTokenId()), 0);
    }

    function testUniV4Mint() public {
        assertEq(c.uniPositionTokenId(), 0);
        int24 delta = 488; // 5% in ticks
        uint256 slippage = 10; // 10%
        _mintUniV4Pos(1 ether, delta, slippage);
        assertGt(c.uniPositionTokenId(), 0);
    }

    function testUniV4Increase() public {
        int24 delta = 488; // 5% in ticks
        uint256 slippage = 10; // 10%
        _mintUniV4Pos(1 ether, delta, slippage);

        uint128 oldLiquidity = uniPosm.getPositionLiquidity(c.uniPositionTokenId());

        (,, uint128 liquidity, uint128 amount0Max, uint128 amount1Max) = _calculateInputs(0.5 ether, delta, slippage);
        c.univ4LiquidityAdd(0, 0, liquidity, amount0Max, amount1Max);

        uint128 newLiquidity = uniPosm.getPositionLiquidity(c.uniPositionTokenId());
        assertGt(newLiquidity, oldLiquidity);
    }

    function testUniV4SwapZeroForOne() public {
        uint256 usdcBalanceOld = collateral.balanceOf(address(c));
        uint128 ethIn = 1 ether;
        uint128 minUsdcOut = 3300e6;
        uint256 amount1 = c.univ4Swap(true, ethIn, minUsdcOut);
        uint256 usdcBalanceNew = collateral.balanceOf(address(c));
        assertEq(usdcBalanceNew - usdcBalanceOld, amount1);
        assertGt(amount1, minUsdcOut);
    }

    function testAaveV3Supply() public {
        assertEq(reserve.balanceOf(address(c)), 0);
        uint256 amount = 1000e6;
        c.aavev3Supply(amount);
        assertGt(reserve.balanceOf(address(c)), 0);
    }

    function testAaveV3Borrow() public {
        uint256 amount = 1000e6;
        c.aavev3Supply(amount);

        // then borrow
        uint256 wethBalanceOld = WETH.balanceOf(address(c));
        uint256 borrowAmount = 0.1 ether;
        c.aavev3Borrow(borrowAmount);
        uint256 wethBalanceNew = WETH.balanceOf(address(c));
        assertEq(wethBalanceNew - wethBalanceOld, borrowAmount);
    }

    function testAaveOraclePrice() public {
        uint256 usdc_p = c.aaveOraclePrice(usdc);
        assertApproxEqRel(usdc_p, 1e8, 5e16);
    }
}
