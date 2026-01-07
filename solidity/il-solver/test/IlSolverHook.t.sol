// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import {Test, console} from "forge-std/Test.sol";
import {IlSolverHook, IWETH, IERC20, IPoolManager, IL2Pool} from "../src/IlSolverHook.sol";

contract IlSolverHookTest is Test {
    address usdc = 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913;
    address weth = 0x4200000000000000000000000000000000000006;

    IWETH WETH;
    IERC20 collateral;
    IERC20 reserve;

    IPoolManager uniPoolManager;

    IL2Pool aavePool;

    IlSolverHook h;
    address owner;

    function setUp() public {
        vm.createSelectFork("base", 39260000);
        owner = makeAddr("owner");

        uniPoolManager = IPoolManager(0x498581fF718922c3f8e6A244956aF099B2652b2b);
        aavePool = IL2Pool(0xA238Dd80C259a72e81d7e4664a9801593F98d1c5);

        WETH = IWETH(weth);
        collateral = IERC20(usdc);
        reserve = IERC20(aavePool.getReserveAToken(usdc));

        h = new IlSolverHook(owner, uniPoolManager, WETH, collateral, aavePool);
    }

    function test_canCallOracle() public {
        assertGt(h.aaveOraclePrice(address(reserve)), 0);
    }
}
