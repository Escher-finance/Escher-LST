// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import {Test, console} from "forge-std/Test.sol";
import {Hooks} from "@uniswap/v4-core/src/libraries/Hooks.sol";
import {HookMiner} from "@uniswap/v4-periphery/src/utils/HookMiner.sol";
import {IlSolverHook, IWETH, IERC20, IPoolManager, IL2Pool, IAaveOracle} from "../src/IlSolverHook.sol";

contract IlSolverHookTest is Test {
    address usdc = 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913;
    address weth = 0x4200000000000000000000000000000000000006;

    IWETH WETH;
    IERC20 collateral;
    IERC20 reserve;

    IPoolManager uniPoolManager;

    IL2Pool aavePool;
    IAaveOracle aaveOracle;

    IlSolverHook h;
    address owner;

    function setUp() public {
        vm.createSelectFork("base", 39260000);
        owner = makeAddr("owner");

        uniPoolManager = IPoolManager(0x498581fF718922c3f8e6A244956aF099B2652b2b);
        aavePool = IL2Pool(0xA238Dd80C259a72e81d7e4664a9801593F98d1c5);
        aaveOracle = IAaveOracle(0x2Cc0Fc26eD4563A5ce5e8bdcfe1A2878676Ae156);

        WETH = IWETH(weth);
        collateral = IERC20(usdc);
        reserve = IERC20(aavePool.getReserveAToken(usdc));

        uint160 flags = uint160(Hooks.AFTER_ADD_LIQUIDITY_FLAG);
        bytes memory constructorArgs = abi.encode(owner, uniPoolManager, WETH, collateral, aavePool, aaveOracle);
        (address hookAddress, bytes32 salt) =
            HookMiner.find(address(this), flags, type(IlSolverHook).creationCode, constructorArgs);

        h = new IlSolverHook{salt: salt}(owner, uniPoolManager, WETH, collateral, aavePool, aaveOracle);
        assertEq(address(h), hookAddress);
    }

    function test_canCallOracle() public {
        uint256 price = h.aaveOraclePrice(address(WETH));
        assertGt(price, 0);
    }
}
