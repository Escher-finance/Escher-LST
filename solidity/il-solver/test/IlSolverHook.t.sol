// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import {Test, console} from "forge-std/Test.sol";
import {Vm} from "forge-std/Vm.sol";
import {Hooks} from "@uniswap/v4-core/src/libraries/Hooks.sol";
import {Actions} from "@uniswap/v4-periphery/src/libraries/Actions.sol";
import {HookMiner} from "@uniswap/v4-periphery/src/utils/HookMiner.sol";
import {
    IlSolverHook,
    IWETH,
    IERC20,
    IPoolManager,
    IL2Pool,
    IAaveOracle,
    IMsgSender,
    IUniversalRouter,
    L2Encoder,
    IPermit2,
    UserData
} from "../src/IlSolverHook.sol";
import {TickMath} from "@uniswap/v4-core/src/libraries/TickMath.sol";
import {LiquidityAmounts} from "@uniswap/v4-core/test/utils/LiquidityAmounts.sol";
import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {Currency, CurrencyLibrary} from "@uniswap/v4-core/src/types/Currency.sol";
import {IHooks} from "@uniswap/v4-core/src/interfaces/IHooks.sol";
import {IStateView} from "@uniswap/v4-periphery/src/interfaces/IStateView.sol";
import {IPositionManager} from "@uniswap/v4-periphery/src/interfaces/IPositionManager.sol";
import {SafeCast} from "@openzeppelin/contracts/utils/math/SafeCast.sol";

using SafeCast for uint256;

contract IlSolverHookTest is Test {
    address usdc = 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913;
    address weth = 0x4200000000000000000000000000000000000006;

    IWETH immutable WETH = IWETH(0x4200000000000000000000000000000000000006);
    IERC20 immutable COLLATERAL = IERC20(usdc);
    IERC20 reserve;

    IPermit2 permit2;

    IPoolManager uniPoolManager;
    IPositionManager uniPosm;
    PoolKey uniPoolKey;
    IUniversalRouter uniRouter;
    IStateView uniStateView;
    uint256 uniPositionTokenId;
    // NOTE: this pool is used only for reference to create the new one
    PoolKey _referencePoolKey;

    IL2Pool aavePool;
    IAaveOracle aaveOracle;
    L2Encoder aaveEncoder;

    IlSolverHook h;
    address owner;

    function setUp() public {
        vm.createSelectFork("base");
        owner = makeAddr("owner");

        _referencePoolKey = PoolKey({
            currency0: CurrencyLibrary.ADDRESS_ZERO,
            currency1: Currency.wrap(usdc),
            fee: 500,
            tickSpacing: 10,
            hooks: IHooks(address(0))
        });

        permit2 = IPermit2(0x000000000022D473030F116dDEE9F6B43aC78BA3);
        uniPoolManager = IPoolManager(0x498581fF718922c3f8e6A244956aF099B2652b2b);
        uniPosm = IPositionManager(0x7C5f5A4bBd8fD63184577525326123B519429bDc);
        uniStateView = IStateView(0xA3c0c9b65baD0b08107Aa264b0f3dB444b867A71);
        uniRouter = IUniversalRouter(0x6fF5693b99212Da76ad316178A184AB56D299b43);
        aavePool = IL2Pool(0xA238Dd80C259a72e81d7e4664a9801593F98d1c5);
        aaveOracle = IAaveOracle(0x2Cc0Fc26eD4563A5ce5e8bdcfe1A2878676Ae156);
        aaveEncoder = L2Encoder(0x39e97c588B2907Fb67F44fea256Ae3BA064207C5);

        reserve = IERC20(aavePool.getReserveAToken(usdc));

        uint160 flags = uint160(Hooks.AFTER_ADD_LIQUIDITY_FLAG);
        bytes memory constructorArgs = abi.encode(
            owner,
            uniPoolManager,
            WETH,
            COLLATERAL,
            permit2,
            _referencePoolKey,
            uniRouter,
            aavePool,
            aaveEncoder,
            aaveOracle
        );
        (address hookAddress, bytes32 salt) =
            HookMiner.find(address(this), flags, type(IlSolverHook).creationCode, constructorArgs);

        // deploy hook contract
        h = new IlSolverHook{salt: salt}(
            owner,
            uniPoolManager,
            WETH,
            COLLATERAL,
            permit2,
            _referencePoolKey,
            uniRouter,
            aavePool,
            aaveEncoder,
            aaveOracle
        );
        assertEq(address(h), hookAddress);

        // verify posm as router
        vm.prank(owner);
        h.toggleVerifiedRouter(IMsgSender(address(uniPosm)));

        // create pool
        uniPoolKey = PoolKey({
            currency0: _referencePoolKey.currency0,
            currency1: _referencePoolKey.currency1,
            fee: _referencePoolKey.fee,
            tickSpacing: _referencePoolKey.tickSpacing,
            hooks: IHooks(hookAddress)
        });

        // fund owner
        deal(owner, 10 ether);
        deal(Currency.unwrap(uniPoolKey.currency1), owner, 50000e6);

        (uint160 sqrtPriceX96,,,) = uniStateView.getSlot0(_referencePoolKey.toId());
        // initialize pool
        uniPoolManager.initialize(uniPoolKey, sqrtPriceX96);
    }

    function test_canCallOracle() public view {
        uint256 price = h.aaveOraclePrice(address(WETH));
        assertGt(price, 0);
    }

    function _univ4LiquidityMintFromAmount0(PoolKey memory key, uint256 amount0, int24 delta, uint256 slippage)
        private
        returns (uint256 used0, uint256 used1)
    {
        (uint160 sqrtPriceX96, int24 tick,,) = uniStateView.getSlot0(key.toId());

        int24 tickSpacing = key.tickSpacing;

        /// forge-lint disable-next-line
        int24 tickLower = ((tick - delta) / tickSpacing) * tickSpacing;
        /// forge-lint disable-next-line
        int24 tickUpper = ((tick + delta) / tickSpacing) * tickSpacing;

        uint160 sqrtPriceAx96 = TickMath.getSqrtPriceAtTick(tickLower);
        uint160 sqrtPriceBx96 = TickMath.getSqrtPriceAtTick(tickUpper);
        uint128 liquidity = LiquidityAmounts.getLiquidityForAmount0(sqrtPriceAx96, sqrtPriceBx96, amount0);

        (uint256 required0, uint256 required1) =
            LiquidityAmounts.getAmountsForLiquidity(sqrtPriceX96, sqrtPriceAx96, sqrtPriceBx96, liquidity);

        uint128 amount0Max = (required0 * (100 + slippage) / 100).toUint128();
        uint128 amount1Max = (required1 * (100 + slippage) / 100).toUint128();

        (used0, used1) =
            _univ4LiquidityMintRaw(uniPoolKey, owner, tickLower, tickUpper, liquidity, amount0Max, amount1Max);
    }

    function _univ4LiquidityMintRaw(
        PoolKey memory k,
        address caller,
        int24 tickLower,
        int24 tickUpper,
        uint256 liquidity,
        uint128 amount0Max,
        uint128 amount1Max
    ) private returns (uint256 used0, uint256 used1) {
        PoolKey memory _key = k;
        address posm = address(uniPosm);

        vm.startPrank(caller);

        IERC20 t1 = IERC20(Currency.unwrap(_key.currency1));
        t1.approve(address(permit2), type(uint128).max);
        permit2.approve(address(t1), posm, type(uint128).max, type(uint48).max);

        bytes memory actions =
            abi.encodePacked(uint8(Actions.MINT_POSITION), uint8(Actions.SETTLE_PAIR), uint8(Actions.SWEEP));

        bytes[] memory params = new bytes[](3);
        params[0] = abi.encode(_key, tickLower, tickUpper, liquidity, amount0Max, amount1Max, caller, bytes(""));
        params[1] = abi.encode(_key.currency0, _key.currency1);
        params[2] = abi.encode(CurrencyLibrary.ADDRESS_ZERO, caller);

        uint256 deadline = block.timestamp;
        IERC20(Currency.unwrap(_key.currency1)).approve(posm, amount1Max);

        uint256 b0Before = _key.currency0.balanceOfSelf();
        uint256 b1Before = _key.currency1.balanceOfSelf();
        uniPositionTokenId = uniPosm.nextTokenId();
        uniPosm.modifyLiquidities{value: amount0Max}(abi.encode(actions, params), deadline);

        uint256 b0After = _key.currency0.balanceOfSelf();
        uint256 b1After = _key.currency1.balanceOfSelf();

        used0 = b0Before + amount0Max - b0After;
        used1 = b1Before + amount1Max - b1After;
    }

    function test_afterAddLiquidityEvent() public {
        uint256 amount0 = 1 ether;
        int24 delta = 488; // 5% in ticks
        uint256 slippage = 10; // 10%

        vm.recordLogs();
        _univ4LiquidityMintFromAmount0(uniPoolKey, amount0, delta, slippage);
        Vm.Log[] memory entries = vm.getRecordedLogs();
        Vm.Log memory dataLog;
        for (uint256 i = 0; i < entries.length; i++) {
            Vm.Log memory log = entries[i];
            if (log.topics[0] == IlSolverHook.AddLiquidityData.selector) {
                dataLog = log;
                break;
            }
        }
        (uint256 borrowedAmountNeeded, uint256 borrowedTokenUsdPrice, uint256 ltv, uint256 collateralAmountNeeded) =
            abi.decode(dataLog.data, (uint256, uint256, uint256, uint256));
        console.log("owner                 ", owner);
        console.log("realSender            ", address(uint160(uint256(dataLog.topics[1]))));
        console.log("borrowedAmountNeeded  ", borrowedAmountNeeded);
        console.log("borrowedTokenUsdPrice ", borrowedTokenUsdPrice);
        console.log("ltv                   ", ltv);
        console.log("collateralAmountNeeded", collateralAmountNeeded);

        UserData memory data = h.getUserData(owner);
        assertGt(data.collateralAmountNeeded, 0);
    }

    function test_loop() public {
        UserData memory data = h.getUserData(owner);
        assert(!data.done);

        uint256 amount0 = 1 ether;
        int24 delta = 488; // 5% in ticks
        uint256 slippage = 10; // 10%
        _univ4LiquidityMintFromAmount0(uniPoolKey, amount0, delta, slippage);

        COLLATERAL.approve(address(h), 10000e8);
        h.loop();

        (
            uint256 totalCollateralBase,
            uint256 totalDebtBase,
            uint256 availableBorrowsBase,
            uint256 currentLiquidationThreshold,
            uint256 ltv,
            uint256 healthFactor
        ) = aavePool.getUserAccountData(address(h));
        console.log("totalCollateralBase", totalCollateralBase);
        console.log("totalDebtBase", totalDebtBase);
        console.log("availableBorrowsBase", availableBorrowsBase);
        console.log("currentLiquidationThreshold", currentLiquidationThreshold);
        console.log("ltv", ltv);
        console.log("healthFactor", healthFactor);

        UserData memory newData = h.getUserData(owner);
        assert(newData.done);
    }
}
