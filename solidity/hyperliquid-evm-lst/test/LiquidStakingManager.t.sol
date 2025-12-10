// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/Test.sol";
import {LiquidStakingManager} from "../src/LiquidStakingManager.sol";
import {Lst} from "../src/tokens/Lst.sol";
import {HyperliquidDelegationManager} from "../src/contracts/HyperliquidDelegationManager.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {DelegationManagerMock} from "./mocks/DelegationManagerMock.sol";
import {Config, Liquidity, BatchStatus, UnbondRequest, UnbondBatch} from "../src/models/State.sol";

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
        bytes memory initData = abi.encodeCall(Lst.initialize, (bob, "eHYPE", "eHP"));
        ERC1967Proxy proxy = new ERC1967Proxy(address(lstImpl), initData);

        lst = Lst(address(proxy));

        LiquidStakingManager liquidStakingManagerImpl = new LiquidStakingManager();
        delegationManager = new DelegationManagerMock();

        address _delegationManager = address(delegationManager);

        bytes memory initLstManagerData =
            abi.encodeCall(LiquidStakingManager.initialize, (bob, address(lst), _delegationManager));
        ERC1967Proxy lstManagerProxy = new ERC1967Proxy(address(liquidStakingManagerImpl), initLstManagerData);

        liquidStakingManager = LiquidStakingManager(payable(address(lstManagerProxy)));

        delegationManager.setLiquidStakingManager(address(liquidStakingManager));

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

        address delegationManagerAddr = liquidStakingManager.getDelegationManager();

        if (delegationManagerAddr == address(0)) {
            vm.expectRevert();
            liquidStakingManager.bond(bondAmount, bob);
        }

        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);

        uint256 bobBalance = liquidStakingManager.getLst().balanceOf(address(bob));
        assertEq(bobBalance, bondAmount);

        // test below min bond amount
        vm.expectRevert();
        liquidStakingManager.bond{value: bondAmount - 10}(bondAmount - 10, bob);
    }

    function testUnbondRequest() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);

        uint256 batchId = liquidStakingManager.getPendingBatchId();

        uint256 minUnbondAmount = liquidStakingManager.getConfig().minUnbondAmount;
        uint256 unbondAmount = minUnbondAmount + 10;
        lst.approve(address(liquidStakingManager), unbondAmount);
        uint256 requestId = liquidStakingManager.unbondRequest(unbondAmount, bob);

        UnbondRequest memory unbondRequest = liquidStakingManager.getUnbondRequest(requestId);
        assertEq(unbondAmount, unbondRequest.shares);
        assertEq(batchId, unbondRequest.batchId);
    }

    function testSubmitBatchAndReceiveBatch() public {
        uint256 batchId = 1; // initial batchId
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);
        uint256 minUnbondAmount = liquidStakingManager.getConfig().minUnbondAmount;
        uint256 unbondAmount = minUnbondAmount + 10;
        lst.approve(address(liquidStakingManager), unbondAmount);
        liquidStakingManager.unbondRequest(unbondAmount, bob);

        // first batch status should be Pending Batch
        UnbondBatch memory pendingBatch = liquidStakingManager.getBatch(batchId);
        assertTrue(pendingBatch.status == BatchStatus.Pending);

        // Submit batch
        liquidStakingManager.submitBatch();

        UnbondBatch memory submittedBatch = liquidStakingManager.getBatch(batchId);
        assertTrue(submittedBatch.status == BatchStatus.Submitted);
        // Fast forward time to allow receiveBatch
        uint256 nextActionTime = block.timestamp + liquidStakingManager.getConfig().undelegatePeriodSeconds;
        vm.warp(nextActionTime + 1);
        liquidStakingManager.receiveBatch(batchId);

        UnbondBatch memory batch = liquidStakingManager.getBatch(batchId);
        assertTrue(batch.status == BatchStatus.Received);
    }

    function testMultipleUsersStakeUnbondClaim() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        vm.startPrank(bob);
        // Bob bonds
        lst.approve(address(liquidStakingManager), bondAmount);
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);
        vm.stopPrank();
        // Alice bonds
        vm.startPrank(alice);
        lst.approve(address(liquidStakingManager), bondAmount);
        liquidStakingManager.bond{value: bondAmount}(bondAmount, alice);
        vm.stopPrank();
        // Both unbond
        vm.startPrank(bob);
        lst.approve(address(liquidStakingManager), bondAmount);
        liquidStakingManager.unbondRequest(bondAmount, bob);
        vm.stopPrank();
        vm.startPrank(alice);
        lst.approve(address(liquidStakingManager), bondAmount);
        liquidStakingManager.unbondRequest(bondAmount, alice);
        vm.stopPrank();
        // Submit batch and receive
        liquidStakingManager.submitBatch();
        uint256 batchId = 1;
        uint256 nextActionTime = block.timestamp + liquidStakingManager.getConfig().undelegatePeriodSeconds;
        vm.warp(nextActionTime + 1);

        UnbondBatch memory batch = liquidStakingManager.getBatch(batchId);
        if (batch.nextActionTime < block.timestamp) {
            vm.deal(address(delegationManager), batch.totalAssets);
        }
        liquidStakingManager.receiveBatch(batchId);
        // Both claim
        vm.startPrank(bob);
        liquidStakingManager.claimUnbond();
        vm.stopPrank();
        vm.startPrank(alice);
        liquidStakingManager.claimUnbond();
        vm.stopPrank();
    }

    function testDoubleUnbondReverts() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);
        lst.approve(address(liquidStakingManager), bondAmount);
        liquidStakingManager.unbondRequest(bondAmount, bob);
        // Try to unbond again with no LST
        vm.expectRevert();
        liquidStakingManager.unbondRequest(bondAmount, bob);
    }

    function testUnbondRequestReverts() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);
        // Unbond below min amount
        lst.approve(address(liquidStakingManager), 1);
        vm.expectRevert();
        liquidStakingManager.unbondRequest(1, bob);
        // Unbond to zero address
        uint256 minUnbondAmount = liquidStakingManager.getConfig().minUnbondAmount;
        lst.approve(address(liquidStakingManager), minUnbondAmount + 10);
        vm.expectRevert();
        liquidStakingManager.unbondRequest(minUnbondAmount + 10, address(0));
    }

    function testSubmitBatchWithEmptyRequests() public {
        // No requests in batch
        vm.expectRevert();
        liquidStakingManager.submitBatch();
    }

    function testReceiveBatchReverts() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);
        uint256 minUnbondAmount = liquidStakingManager.getConfig().minUnbondAmount;
        uint256 unbondAmount = minUnbondAmount + 10;
        lst.approve(address(liquidStakingManager), unbondAmount);
        liquidStakingManager.unbondRequest(unbondAmount, bob);
        liquidStakingManager.submitBatch();
        uint256 batchId = 1;
        // Try to receive before time
        vm.expectRevert();
        liquidStakingManager.receiveBatch(batchId);
    }

    function testBatchAndRequestGetters() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);
        uint256 minUnbondAmount = liquidStakingManager.getConfig().minUnbondAmount;
        uint256 unbondAmount = minUnbondAmount + 10;
        lst.approve(address(liquidStakingManager), unbondAmount);
        liquidStakingManager.unbondRequest(unbondAmount, bob);
        uint256 batchId = liquidStakingManager.getPendingBatchId();
        uint256[] memory reqIds = liquidStakingManager.getBatchRequestIds(batchId);
        assertGt(reqIds.length, 0);
        uint256[] memory userReqIds = liquidStakingManager.getUserRequestIds(bob);
        assertGt(userReqIds.length, 0);
        uint256 nextReqId = liquidStakingManager.getNextRequestId();
        assertGt(nextReqId, 0);
        // Get batch and request
        UnbondBatch memory batch = liquidStakingManager.getBatch(batchId);
        UnbondRequest memory req = liquidStakingManager.getUnbondRequest(reqIds[0]);
        assertEq(batch.batchId, batchId);
        assertEq(req.user, bob);
    }

    function testStateAfterOperations() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);
        lst.approve(address(liquidStakingManager), bondAmount);
        liquidStakingManager.unbondRequest(bondAmount, bob);
        liquidStakingManager.submitBatch();
        uint256 batchId = 1;
        uint256 nextActionTime = block.timestamp + liquidStakingManager.getConfig().undelegatePeriodSeconds;
        vm.warp(nextActionTime + 1);
        liquidStakingManager.receiveBatch(batchId);
        UnbondBatch memory batch = liquidStakingManager.getBatch(batchId);
        assertTrue(batch.status == BatchStatus.Received);
        assertEq(batch.totalShares, bondAmount);
        assertGt(batch.totalAssets, 0);
    }

    function testBatchLifecycleMultipleRequests() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        // Bob bonds and unbonds
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);
        lst.approve(address(liquidStakingManager), bondAmount);
        liquidStakingManager.unbondRequest(bondAmount, bob);
        // Alice bonds and unbonds
        vm.startPrank(alice);
        lst.approve(address(liquidStakingManager), bondAmount);
        liquidStakingManager.bond{value: bondAmount}(bondAmount, alice);
        liquidStakingManager.unbondRequest(bondAmount, alice);
        vm.stopPrank();
        // Submit batch and receive
        liquidStakingManager.submitBatch();
        uint256 batchId = 1;
        uint256 nextActionTime = block.timestamp + liquidStakingManager.getConfig().undelegatePeriodSeconds;
        vm.warp(nextActionTime + 1);
        // Check batch status and requests
        UnbondBatch memory batch = liquidStakingManager.getBatch(batchId);
        if (batch.nextActionTime < block.timestamp) {
            vm.deal(address(delegationManager), batch.totalAssets);
        }

        liquidStakingManager.receiveBatch(batchId);
        UnbondBatch memory receivedBatch = liquidStakingManager.getBatch(batchId);
        assertTrue(receivedBatch.status == BatchStatus.Received);
        assertEq(receivedBatch.requestIds.length, 2);
    }
}
