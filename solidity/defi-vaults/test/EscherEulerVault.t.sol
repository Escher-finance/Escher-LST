// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {EscherEulerVault, IEVault, IERC20} from "../src/EscherEulerVault.sol";

contract EscherEulerVaultTest is Test {
    EscherEulerVault vault;
    IERC20 underylingAsset;
    address owner;
    address user;

    function setUp() public {
        vm.createSelectFork("mainnet", 23432500);
        IEVault eulerVault = IEVault(0x3573A84Bee11D49A1CbCe2b291538dE7a7dD81c6);
        underylingAsset = IERC20(eulerVault.asset());
        owner = makeAddr("owner");
        user = makeAddr("user");
        vault = new EscherEulerVault(owner, "share token", "shareTkn", underylingAsset, eulerVault);
        assert(address(vault) != address(0));

        uint256 dealAmount = 100 ether;
        deal(address(underylingAsset), owner, dealAmount);
        deal(address(underylingAsset), user, dealAmount);
        assertGe(underylingAsset.balanceOf(owner), dealAmount);
        assertGe(underylingAsset.balanceOf(user), dealAmount);
    }

    function test_lending() public {
        assertEq(vault.convertToShares(10000), 10000, "should have 1:1 ratio initially");

        vm.startPrank(user);

        assertEq(vault.totalSupply(), 0);
        assertEq(vault.totalAssets(), 0);
        uint256 depositAmount = 500;

        // deposit

        underylingAsset.approve(address(vault), depositAmount);
        vault.deposit(depositAmount, user);

        assertGe(vault.convertToShares(10000), 10000);
        assertEq(vault.balanceOf(user), depositAmount);
        assertEq(vault.totalSupply(), depositAmount);
        assertApproxEqRel(vault.totalAssets(), depositAmount, 0.01 ether);
        assertGe(vault.s_eulerVault().balanceOf(address(vault)), 0);

        // redeem

        uint256 redeemAmount = depositAmount / 2;
        vault.approve(address(vault), redeemAmount);
        vault.redeem(redeemAmount, user, user);
        assertGe(vault.convertToShares(10000), 10000);
        assertEq(vault.balanceOf(user), redeemAmount);
        assertEq(vault.totalSupply(), depositAmount - redeemAmount);
        assertApproxEqRel(vault.totalAssets(), depositAmount - redeemAmount, 0.01 ether);
        assertGe(vault.s_eulerVault().balanceOf(address(vault)), 0);
    }
}
