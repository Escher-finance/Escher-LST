// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import {Test, console} from "forge-std/Test.sol";
import {IlSolver} from "../src/IlSolver.sol";

contract IlSolverTest is Test {
    IlSolver c;
    address owner;

    function setUp() public {
        vm.createSelectFork("mainnet", 23968000);
        owner = makeAddr("owner");
        deal(owner, 1 ether);
    }
}
