// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;

import {Test, console} from "forge-std/Test.sol";
import {LiquidStakingManager} from "../src/LiquidStakingManager.sol";
import {DelegationManager} from "../src/DelegationManager.sol";
import {DepositToken} from "../src/tokens/DepositToken.sol"; // Adjust the import path as necessary

contract LiquidStakingManagerTest is Test {
    LiquidStakingManager public liquidStakingManager;
    DepositToken public asset;

    address public bob = makeAddr("bob");
    address public alice = makeAddr("alice");
    uint256 public STARTING_BALANCE = 10000;

    function setUp() public {
        vm.startPrank(bob);
        asset = new DepositToken("Bebe", "ubbn");
        asset.mint(bob, 1000000);
        asset.mint(alice, 1000000);

        liquidStakingManager = new LiquidStakingManager(bob, address(asset), "etk", "eToken");

        vm.deal(bob, STARTING_BALANCE);
        vm.deal(alice, STARTING_BALANCE);
    }

    function testOwner() public view {
        address owner = liquidStakingManager.owner();
        assertEq(bob, owner);
    }

    function testBond() public {
        vm.startPrank(bob);

        uint256 managerBalance = asset.balanceOf(address(liquidStakingManager));
        assertEq(managerBalance, 0);

        uint256 bondAmount = liquidStakingManager.getConfig().minBondAmount + 1;
        asset.approve(address(liquidStakingManager), bondAmount);

        uint256 bond = liquidStakingManager.bondRate();
        console.log("bond rate", bond);

        liquidStakingManager.bond(bondAmount, bob);

        uint256 afterManagerBalance = asset.balanceOf(address(liquidStakingManager));
        assertEq(afterManagerBalance, bondAmount);

        uint256 bobBalance = liquidStakingManager.getLst().balanceOf(address(bob));
        assertEq(bobBalance, bondAmount);

        vm.expectRevert();
        liquidStakingManager.bond(bondAmount, bob);

        // bond with amount below min bond amount
        vm.expectRevert();
        liquidStakingManager.bond(bondAmount - 10, bob);
    }
}
