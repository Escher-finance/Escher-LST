// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.28;

import {Test, console} from "forge-std/Test.sol";
import {ILiquidStakingManager} from "../src/interfaces/ILiquidStakingManager.sol";
import {LiquidStakingManager} from "../src/contracts/LiquidStakingManager.sol";
import {Lst} from "../src/tokens/Lst.sol";
import {DelegatorSummary, InitializeLstManagerPayload, Rate} from "../src/models/Type.sol";
import {HyperliquidDelegationManager} from "../src/contracts/HyperliquidDelegationManager.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {DelegationManagerMock} from "./mocks/DelegationManagerMock.sol";
import {Config, Liquidity, BatchStatus, UnbondRequest, UnbondBatch} from "../src/models/State.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin-upgradeable/contracts/proxy/utils/UUPSUpgradeable.sol";

contract LiquidStakingManagerTest is Test {
    LiquidStakingManager public liquidStakingManager;
    Lst public lst;
    DelegationManagerMock public delegationManager;
    LiquidStakingManager public stableLstManager;
    ERC1967Proxy lstManagerProxy;

    address public bob = makeAddr("bob");
    address public alice = makeAddr("alice");

    uint256 public constant CORE_TO_EVM = 10 ** 10;
    uint256 public STARTING_BALANCE = 10000 * CORE_TO_EVM;
    uint256 public SCALING_FACTOR = 10 ** 18;

    function setUp() public {
        vm.startPrank(bob);
        Lst lstImpl = new Lst();
        bytes memory initData = abi.encodeCall(Lst.initialize, (bob, "eHYPE", "eHP"));
        ERC1967Proxy proxy = new ERC1967Proxy(address(lstImpl), initData);

        lst = Lst(address(proxy));

        LiquidStakingManager liquidStakingManagerImpl = new LiquidStakingManager();
        delegationManager = new DelegationManagerMock();

        address _delegationManager = address(delegationManager);
        InitializeLstManagerPayload memory payload = InitializeLstManagerPayload({
            chainName: "hyperliquid",
            initialOwner: bob,
            lstAddress: address(lst),
            delegationManagerAddress: _delegationManager,
            minBondAmount: 1000 * CORE_TO_EVM,
            minUnbondAmount: 1000 * CORE_TO_EVM,
            batchPeriodSeconds: 300,
            undelegatePeriodSeconds: 300
        });
        bytes memory initLstManagerData = abi.encodeCall(LiquidStakingManager.initialize, (payload));
        lstManagerProxy = new ERC1967Proxy(address(liquidStakingManagerImpl), initLstManagerData);

        liquidStakingManager = LiquidStakingManager(payable(address(lstManagerProxy)));

        delegationManager.setLiquidStakingManager(address(liquidStakingManager));

        InitializeLstManagerPayload memory payload2 = InitializeLstManagerPayload({
            chainName: "stable",
            initialOwner: bob,
            lstAddress: address(lst),
            delegationManagerAddress: _delegationManager,
            minBondAmount: 1000 * CORE_TO_EVM,
            minUnbondAmount: 1000 * CORE_TO_EVM,
            batchPeriodSeconds: 300,
            undelegatePeriodSeconds: 300
        });
        bytes memory initLstManagerData2 = abi.encodeCall(LiquidStakingManager.initialize, (payload2));
        ERC1967Proxy lstManagerProxy2 = new ERC1967Proxy(address(liquidStakingManagerImpl), initLstManagerData2);

        stableLstManager = LiquidStakingManager(payable(address(lstManagerProxy2)));

        vm.deal(bob, STARTING_BALANCE);
        vm.deal(alice, STARTING_BALANCE);
    }

    function testOwner() public view {
        address owner = liquidStakingManager.owner();
        assertEq(bob, owner);
    }

    function testRate() public view {
        Rate memory rate = liquidStakingManager.rate();
        assertEq(rate.bondRate, SCALING_FACTOR);
        assertEq(rate.unbondRate, SCALING_FACTOR);
    }

    function testTransferOwnership() public {
        address owner = lst.owner();
        assertEq(bob, owner);

        lst.transferOwnership(address(liquidStakingManager));

        liquidStakingManager.acceptOwnershipTransfer();

        address newOwner = lst.owner();
        assertEq(address(liquidStakingManager), newOwner);
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

        Liquidity memory summary = liquidStakingManager.getLiquidity();
        assertEq(summary.totalDelegated, bondAmount);
        assertEq(summary.totalLst, bobBalance);

        console.log("summary.totalDelegated", summary.totalDelegated);
        console.log("summary.totalLst", summary.totalLst);

        // test below min bond amount
        vm.expectRevert();
        liquidStakingManager.bond{value: bondAmount - 10}(bondAmount - 10, bob);
    }

    function testUnbondRequestSuccess() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);

        uint256 batchId = liquidStakingManager.getCurrentBatchId();

        uint256 minUnbondAmount = liquidStakingManager.getConfig().minUnbondAmount;
        uint256 unbondAmount = minUnbondAmount + 10;
        lst.approve(address(liquidStakingManager), unbondAmount);
        uint256 requestId = liquidStakingManager.unbondRequest(unbondAmount);
        assertEq(requestId, 1);

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
        liquidStakingManager.unbondRequest(unbondAmount);

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
        liquidStakingManager.moveBatch(batchId);
        liquidStakingManager.receiveBatch(batchId);

        UnbondBatch memory batch = liquidStakingManager.getBatch(batchId);
        assertTrue(batch.status == BatchStatus.Received);
    }

    function testMultipleUsersStakeUnbondClaim() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        vm.startPrank(bob);
        // Bob bonds
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);
        vm.stopPrank();
        // Alice bonds
        vm.startPrank(alice);
        liquidStakingManager.bond{value: bondAmount}(bondAmount, alice);
        vm.stopPrank();
        // Both unbond
        vm.startPrank(bob);
        lst.approve(address(liquidStakingManager), bondAmount);
        liquidStakingManager.unbondRequest(bondAmount);
        vm.stopPrank();
        vm.startPrank(alice);
        lst.approve(address(liquidStakingManager), bondAmount);
        liquidStakingManager.unbondRequest(bondAmount);
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
        liquidStakingManager.moveBatch(batchId);
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
        liquidStakingManager.unbondRequest(bondAmount);
        // Try to unbond again with no LST
        vm.expectRevert();
        liquidStakingManager.unbondRequest(bondAmount);
    }

    function testUnbondRequestReverts() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);
        // Unbond below min amount
        lst.approve(address(liquidStakingManager), 1);
        vm.expectRevert(bytes("unbond should be more than min unbond amount"));
        liquidStakingManager.unbondRequest(1);
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
        liquidStakingManager.unbondRequest(unbondAmount);
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
        liquidStakingManager.unbondRequest(unbondAmount);
        uint256 batchId = liquidStakingManager.getCurrentBatchId();
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
        assertEq(req.sender, bob);
    }

    function testStateAfterOperations() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = (minBondAmount + 100);
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);
        lst.approve(address(liquidStakingManager), bondAmount);
        liquidStakingManager.unbondRequest(bondAmount);
        liquidStakingManager.submitBatch();
        uint256 batchId = 1;
        uint256 nextActionTime = block.timestamp + liquidStakingManager.getConfig().undelegatePeriodSeconds;
        vm.warp(nextActionTime + 1);
        liquidStakingManager.moveBatch(batchId);
        liquidStakingManager.receiveBatch(batchId);
        UnbondBatch memory batch = liquidStakingManager.getBatch(batchId);
        assertTrue(batch.status == BatchStatus.Received);
        assertEq(batch.totalShares, bondAmount);
    }

    function testBatchLifecycleMultipleRequests() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        // Bob bonds and unbonds
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);
        lst.approve(address(liquidStakingManager), bondAmount);
        liquidStakingManager.unbondRequest(bondAmount);
        // Alice bonds and unbonds
        vm.startPrank(alice);
        lst.approve(address(liquidStakingManager), bondAmount);
        liquidStakingManager.bond{value: bondAmount}(bondAmount, alice);

        liquidStakingManager.unbondRequest(bondAmount);
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
        liquidStakingManager.moveBatch(batchId);
        liquidStakingManager.receiveBatch(batchId);
        UnbondBatch memory receivedBatch = liquidStakingManager.getBatch(batchId);
        assertTrue(receivedBatch.status == BatchStatus.Received);
        assertEq(receivedBatch.requestIds.length, 2);
    }

    function testUnbondTransfersShares() public {
        uint256 minBond = liquidStakingManager.getConfig().minUnbondAmount;
        uint256 bondAmount = minBond + 200;

        uint256 minUnbond = liquidStakingManager.getConfig().minUnbondAmount;
        uint256 unbondAmount = minUnbond + 50;

        // Bob bonds and receives LST
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);
        assertEq(lst.balanceOf(bob), bondAmount);

        // Approve manager to transfer shares and unbond
        lst.approve(address(liquidStakingManager), unbondAmount);
        uint256 reqId = liquidStakingManager.unbondRequest(unbondAmount);

        // Verify balances: bob decreased, contract increased
        assertEq(lst.balanceOf(bob), bondAmount - unbondAmount);
        assertEq(lst.balanceOf(address(liquidStakingManager)), unbondAmount);

        // Verify request stored correctly
        UnbondRequest memory req = liquidStakingManager.getUnbondRequest(reqId);
        assertEq(req.shares, unbondAmount);
        assertEq(req.sender, bob);
    }

    function testUnbondWithoutApprovalReverts() public {
        uint256 minBond = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBond + 100;
        uint256 unbondAmount = 10;

        // Bob bonds
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);
        assertEq(lst.balanceOf(bob), bondAmount);

        // Do NOT approve: expect revert when trying to unbond
        vm.expectRevert();
        liquidStakingManager.unbondRequest(unbondAmount);
    }

    function testClaimUnbondSuccess() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);

        uint256 minUnbondAmount = liquidStakingManager.getConfig().minUnbondAmount;
        uint256 unbondAmount = minUnbondAmount + 10;
        lst.approve(address(liquidStakingManager), unbondAmount);
        uint256 reqId = liquidStakingManager.unbondRequest(unbondAmount);
        assertEq(reqId, 1);

        liquidStakingManager.submitBatch();
        uint256 batchId = 1;
        uint256 nextActionTime = block.timestamp + liquidStakingManager.getConfig().undelegatePeriodSeconds;
        vm.warp(nextActionTime + 1);

        UnbondBatch memory batch = liquidStakingManager.getBatch(batchId);
        if (batch.nextActionTime < block.timestamp) {
            vm.deal(address(delegationManager), batch.totalAssets);
        }
        liquidStakingManager.moveBatch(batchId);
        liquidStakingManager.receiveBatch(batchId);
        // Claim all
        liquidStakingManager.claimUnbond();
        // Should revert if called again
        vm.expectRevert();
        liquidStakingManager.claimUnbond();
    }

    function testClaimUnbondRequestSuccess() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);

        uint256 minUnbondAmount = liquidStakingManager.getConfig().minUnbondAmount;
        uint256 unbondAmount = minUnbondAmount + 10;
        lst.approve(address(liquidStakingManager), unbondAmount);
        uint256 reqId = liquidStakingManager.unbondRequest(unbondAmount);
        assertEq(reqId, 1);

        uint256 balance1 = bob.balance;

        liquidStakingManager.submitBatch();
        uint256 batchId = 1;
        uint256 nextActionTime = block.timestamp + liquidStakingManager.getConfig().undelegatePeriodSeconds;
        vm.warp(nextActionTime + 1);

        UnbondBatch memory batch = liquidStakingManager.getBatch(batchId);
        if (batch.nextActionTime < block.timestamp) {
            vm.deal(address(delegationManager), batch.totalAssets);
        }
        liquidStakingManager.moveBatch(batchId);
        liquidStakingManager.receiveBatch(batchId);
        liquidStakingManager.claimUnbondRequest(reqId);

        uint256 balance2 = bob.balance;
        assertEq(balance2, balance1 + minBondAmount);
    }

    function testClaimUnbondRequestReverts() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);

        uint256 minUnbondAmount = liquidStakingManager.getConfig().minUnbondAmount;
        uint256 unbondAmount = minUnbondAmount + 10;
        lst.approve(address(liquidStakingManager), unbondAmount);
        liquidStakingManager.unbondRequest(unbondAmount);
        liquidStakingManager.submitBatch();
        uint256 batchId = 1;
        uint256 nextActionTime = block.timestamp + liquidStakingManager.getConfig().undelegatePeriodSeconds;
        vm.warp(nextActionTime + 1);

        UnbondBatch memory batch = liquidStakingManager.getBatch(batchId);
        if (batch.nextActionTime < block.timestamp) {
            vm.deal(address(delegationManager), batch.totalAssets);
        }

        liquidStakingManager.moveBatch(batchId);

        vm.expectRevert(bytes("batch not yet received"));
        liquidStakingManager.claimUnbondRequest(1);

        liquidStakingManager.receiveBatch(batchId);

        // Invalid requestId
        vm.expectRevert();
        liquidStakingManager.claimUnbondRequest(999);
        // Not owner
        vm.startPrank(alice);
        uint256[] memory reqIds = liquidStakingManager.getUserRequestIds(bob);
        if (reqIds.length > 0) {
            vm.expectRevert();
            liquidStakingManager.claimUnbondRequest(reqIds[0]);
        }
    }

    function testReceiveBatchIncorrectBatchStatus() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);

        uint256 minUnbondAmount = liquidStakingManager.getConfig().minUnbondAmount;
        uint256 unbondAmount = minUnbondAmount + 10;
        lst.approve(address(liquidStakingManager), unbondAmount);
        liquidStakingManager.unbondRequest(unbondAmount);

        uint256 batchId = 1;

        vm.expectRevert(ILiquidStakingManager.IncorrectBatchStatus.selector);
        liquidStakingManager.receiveBatch(batchId);

        liquidStakingManager.submitBatch();
        uint256 nextActionTime = block.timestamp + liquidStakingManager.getConfig().undelegatePeriodSeconds;
        vm.warp(nextActionTime + 1);
        vm.expectRevert(ILiquidStakingManager.IncorrectBatchStatus.selector);
        liquidStakingManager.receiveBatch(batchId);
    }

    function testIncorrectBatchStatus() public {
        delegationManager.setLiquidStakingManager(address(stableLstManager));
        uint256 minBondAmount = stableLstManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        stableLstManager.bond{value: bondAmount}(bondAmount, bob);

        uint256 minUnbondAmount = stableLstManager.getConfig().minUnbondAmount;
        uint256 unbondAmount = minUnbondAmount + 10;
        lst.approve(address(stableLstManager), unbondAmount);
        stableLstManager.unbondRequest(unbondAmount);

        uint256 batchId = 1;
        vm.expectRevert(ILiquidStakingManager.IncorrectBatchStatus.selector);
        stableLstManager.receiveBatch(batchId);
    }

    function testUnbondRequestFail() public {
        uint256 minBondAmount = liquidStakingManager.getConfig().minBondAmount;
        uint256 bondAmount = minBondAmount + 100;
        liquidStakingManager.bond{value: bondAmount}(bondAmount, bob);

        uint256 minUnbondAmount = liquidStakingManager.getConfig().minUnbondAmount;
        uint256 unbondAmount = minUnbondAmount + 10;

        vm.expectRevert();
        liquidStakingManager.unbondRequest(unbondAmount);
    }

    function testRevertIfNonOwnerUpgrades() public {
        address newImplementation = address(new LiquidStakingManager());

        vm.startPrank(alice);
        // The upgradeToAndCall function is the entry point for UUPS upgrades
        vm.expectRevert(abi.encodeWithSignature("OwnableUnauthorizedAccount(address)", alice));
        UUPSUpgradeable(address(lstManagerProxy)).upgradeToAndCall(newImplementation, "");
        vm.stopPrank();
    }

    function testOwnerCanUpgrade() public {
        address newImplementation = address(new LiquidStakingManager());

        vm.startPrank(bob);
        UUPSUpgradeable(address(lstManagerProxy)).upgradeToAndCall(newImplementation, "");

        // Verify the implementation slot was updated
        bytes32 slot = bytes32(uint256(keccak256("eip1967.proxy.implementation")) - 1);
        address currentImpl = address(uint160(uint256(vm.load(address(lstManagerProxy), slot))));
        assertEq(currentImpl, newImplementation);
    }
}
