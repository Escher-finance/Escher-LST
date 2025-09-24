// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {EscherEulerVault, IEVault, IERC20, IEthereumVaultConnector} from "../src/EscherEulerVault.sol";

contract EscherEulerVaultTest is Test {
    EscherEulerVault vault;
    IERC20 underylingAsset;
    IEVault eulerVault;
    IEVault collateralVault1;
    IEVault collateralVault2;
    address owner;
    address user;

    function compareArrays(address[] memory a, address[] memory b) internal pure returns (bool) {
        if (a.length != b.length) return false;
        for (uint256 i = 0; i < a.length; i++) {
            if (a[i] != b[i]) return false;
        }
        return true;
    }

    function setUp() public {
        vm.createSelectFork("mainnet", 23432500);
        eulerVault = IEVault(0x3573A84Bee11D49A1CbCe2b291538dE7a7dD81c6);
        collateralVault1 = IEVault(0xE415952f5ee06f8A548F4f7D5bE18FBf144b4E4D);
        collateralVault2 = IEVault(0xe0a80d35bB6618CBA260120b279d357978c42BCE);
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
        deal(collateralVault2.asset(), owner, dealAmount);
        deal(collateralVault2.asset(), user, dealAmount);

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
        vm.startPrank(owner);
        uint256 depositAmount = 100000;
        address[] memory expectedCollaterals = new address[](2);

        // add 1st collateral
        compareArrays(vault.collaterals(), expectedCollaterals);
        IERC20(collateralVault1.asset()).approve(address(collateralVault1), depositAmount);
        collateralVault1.deposit(depositAmount, address(vault));
        vault.addCollateral(collateralVault1);
        expectedCollaterals[0] = address(collateralVault1);
        compareArrays(vault.collaterals(), expectedCollaterals);

        // add 2nd collateral
        IERC20(collateralVault2.asset()).approve(address(collateralVault2), depositAmount);
        collateralVault2.deposit(depositAmount, address(vault));
        vault.addCollateral(collateralVault2);
        expectedCollaterals[1] = address(collateralVault2);
        compareArrays(vault.collaterals(), expectedCollaterals);

        // borrow
        vault.borrow(50);
    }
}
