// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {EscherEulerVault, IEVault, IERC20, IEthereumVaultConnector} from "../src/EscherEulerVault.sol";

contract EscherEulerVaultTest is Test {
    EscherEulerVault vault;
    IERC20 underylingAsset;
    IEVault eulerVault;
    IEVault collateralVault1;
    address owner;
    address user;

    function setUp() public {
        vm.createSelectFork("mainnet", 23432500);
        eulerVault = IEVault(0x3573A84Bee11D49A1CbCe2b291538dE7a7dD81c6);
        collateralVault1 = IEVault(0x797DD80692c3b2dAdabCe8e30C07fDE5307D48a9);
        underylingAsset = IERC20(eulerVault.asset());
        owner = makeAddr("owner");
        user = makeAddr("user");
        vault = new EscherEulerVault(owner, "share token", "shareTkn", underylingAsset, eulerVault);
        assert(address(vault) != address(0));

        uint256 dealAmount = 100 ether;
        deal(address(underylingAsset), owner, dealAmount);
        deal(address(underylingAsset), user, dealAmount);
        deal(collateralVault1.asset(), owner, dealAmount);
        deal(collateralVault1.asset(), user, dealAmount);

        IEthereumVaultConnector evc = IEthereumVaultConnector(payable(0x0C9a3dd6b8F28529d72d7f9cE918D493519EE383));
        vm.prank(owner);
        vault.updateEulerEVC(evc);
    }

    function test_lending() public {
        vm.startPrank(user);

        assertEq(vault.convertToShares(10000), 10000, "should have 1:1 ratio initially");

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
        assertGe(eulerVault.balanceOf(address(vault)), 0);

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

    function test_collateralsAndBorrowing() public {
        vm.startPrank(user);

        // make a deposit just to make the vault have some tokens
        uint256 depositAmount = 500;
        underylingAsset.approve(address(vault), depositAmount);
        vault.deposit(depositAmount, user);

        uint256 collateralAmount = 15000;
        uint256 borrowAmount = 10000;

        assertEq(vault.s_eulerEVC().isCollateralEnabled(address(vault), address(collateralVault1)), false);

        // user deposits collateral
        IERC20(collateralVault1.asset()).approve(address(collateralVault1), collateralAmount);
        collateralVault1.deposit(collateralAmount, address(vault));

        vm.startPrank(owner);

        // owner adds collateral
        vault.addCollateral(collateralVault1);

        assertEq(vault.s_eulerEVC().isCollateralEnabled(address(vault), address(collateralVault1)), true);
        assertEq(vault.s_eulerEVC().isControllerEnabled(address(vault), address(eulerVault)), true);

        (uint256 collateralValue,) = eulerVault.accountLiquidity(address(vault), false);
        assertGt(collateralValue, 0);

        // borrow
        assertApproxEqRel(vault.totalAssets(), depositAmount, 0.01 ether);
        vault.borrow(borrowAmount);
        assertApproxEqRel(vault.totalAssets(), depositAmount + borrowAmount, 0.01 ether);
    }
}
