// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/Test.sol";
import {LiquidStakingManager} from "../src/LiquidStakingManager.sol";
import {Lst} from "../src/tokens/Lst.sol";
import {DelegationManager} from "../src/DelegationManager.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {DelegationManagerMock} from "./mocks/DelegationManagerMock.sol";

contract LiquidStakingManagerTest is Test {
    LiquidStakingManager public liquidStakingManager;
    Lst public lst;
    DelegationManagerMock public delegationManager;

    address public bob = makeAddr("bob");
    address public alice = makeAddr("alice");
    uint256 public STARTING_BALANCE = 10000;

    function setUp() public {
        vm.startPrank(bob);
        Lst lstImpl = new Lst();
        bytes memory initData = abi.encodeCall(
            Lst.initialize,
            (bob, "eHYPE", "eHP")
        );
        ERC1967Proxy proxy = new ERC1967Proxy(address(lstImpl), initData);

        lst = Lst(address(proxy));

        LiquidStakingManager liquidStakingManagerImpl = new LiquidStakingManager();

        bytes memory initLstManagerData = abi.encodeCall(
            LiquidStakingManager.initialize,
            (bob, address(lst))
        );
        ERC1967Proxy lstManagerProxy = new ERC1967Proxy(
            address(liquidStakingManagerImpl),
            initLstManagerData
        );

        liquidStakingManager = LiquidStakingManager(address(lstManagerProxy));

        delegationManager = new DelegationManagerMock();

        vm.deal(bob, STARTING_BALANCE);
        vm.deal(alice, STARTING_BALANCE);
    }

    function testOwner() public view {
        address owner = liquidStakingManager.owner();
        assertEq(bob, owner);
    }

    function testBond() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 1;

        uint256 bond = liquidStakingManager.bondRate();
        console.log("bond rate", bond);

        address delegationManagerAddr = liquidStakingManager
            .getDelegationManager();

        if (delegationManagerAddr == address(0)) {
            vm.expectRevert();
            liquidStakingManager.bond(bondAmount, bob);
        }

        liquidStakingManager.setDelegationManager(address(delegationManager));

        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);

        uint256 bobBalance = liquidStakingManager.getLst().balanceOf(
            address(bob)
        );
        assertEq(bobBalance, bondAmount);

        vm.expectRevert();
        // test below min bond amount
        liquidStakingManager.bond{value: bondAmount - 10}(bondAmount - 10, bob);
    }
}
