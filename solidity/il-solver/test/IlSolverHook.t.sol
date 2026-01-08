// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import {Test, console} from "forge-std/Test.sol";
import {Hooks} from "@uniswap/v4-core/src/libraries/Hooks.sol";
import {HookMiner} from "@uniswap/v4-periphery/src/utils/HookMiner.sol";
import {IlSolverHook, IWETH, IERC20, IPoolManager, IL2Pool, IAaveOracle} from "../src/IlSolverHook.sol";
import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {Currency, CurrencyLibrary} from "@uniswap/v4-core/src/types/Currency.sol";
import {IHooks} from "@uniswap/v4-core/src/interfaces/IHooks.sol";
import {IStateView} from "@uniswap/v4-periphery/src/interfaces/IStateView.sol";

contract IlSolverHookTest is Test {
    address usdc = 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913;
    address weth = 0x4200000000000000000000000000000000000006;

    IWETH WETH;
    IERC20 collateral;
    IERC20 reserve;

    IPoolManager uniPoolManager;
    PoolKey uniPoolKey;
    IStateView uniStateView;
    // NOTE: this pool is used only for reference to create the new one
    PoolKey _referencePoolKey;

    IL2Pool aavePool;
    IAaveOracle aaveOracle;

    IlSolverHook h;
    address owner;

    function setUp() public {
        vm.createSelectFork("base", 39260000);
        owner = makeAddr("owner");

        _referencePoolKey = PoolKey({
            currency0: CurrencyLibrary.ADDRESS_ZERO,
            currency1: Currency.wrap(usdc),
            fee: 500,
            tickSpacing: 10,
            hooks: IHooks(address(0))
        });

        uniPoolManager = IPoolManager(0x498581fF718922c3f8e6A244956aF099B2652b2b);
        uniStateView = IStateView(0xA3c0c9b65baD0b08107Aa264b0f3dB444b867A71);
        aavePool = IL2Pool(0xA238Dd80C259a72e81d7e4664a9801593F98d1c5);
        aaveOracle = IAaveOracle(0x2Cc0Fc26eD4563A5ce5e8bdcfe1A2878676Ae156);

        WETH = IWETH(weth);
        collateral = IERC20(usdc);
        reserve = IERC20(aavePool.getReserveAToken(usdc));

        uint160 flags = uint160(Hooks.AFTER_ADD_LIQUIDITY_FLAG);
        bytes memory constructorArgs = abi.encode(owner, uniPoolManager, WETH, collateral, aavePool, aaveOracle);
        (address hookAddress, bytes32 salt) =
            HookMiner.find(address(this), flags, type(IlSolverHook).creationCode, constructorArgs);

        // deploy hook contract
        h = new IlSolverHook{salt: salt}(owner, uniPoolManager, WETH, collateral, aavePool, aaveOracle);
        assertEq(address(h), hookAddress);

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

    function test_canCallOracle() public {
        uint256 price = h.aaveOraclePrice(address(WETH));
        assertGt(price, 0);
    }
}
