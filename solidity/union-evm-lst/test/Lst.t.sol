// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {Lst, InitializePayload} from "../src/Lst.sol";
import {LSTProxy} from "../src/LSTProxy.sol";
import {eU} from "../src/eU.sol";
import {BaseToken} from "../src/U.sol";
import {HubBatch, BatchStatus, Config} from "@common/zkgm-core/Types.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

uint256 constant SCALING_FACTOR = 10 ** 18;

contract LstTest is Test {
    Lst public lst;
    LSTProxy lstProxy;
    BaseToken public uToken;
    eU public eu;
    address owner;

    address public ucs03 = 0x5FbE74A283f7954f10AA04C2eDf55578811aeb03;

    function setUp() public {
        address euimplementation = address(new eU());

        owner = makeAddr("owner");
        vm.startPrank(owner);

        console.log("eu implementation deployed at:", euimplementation);

        bytes memory euinitializeData = abi.encodeWithSelector(eu.initialize.selector, owner, ucs03);

        ERC1967Proxy proxy = new ERC1967Proxy(euimplementation, euinitializeData);
        console.log("eu Proxy deployed at:", address(proxy));

        eu = eU(address(proxy));

        address UImplementation = address(new BaseToken());

        bytes memory uTokenInitializeData = abi.encodeWithSelector(BaseToken.initialize.selector, owner);

        ERC1967Proxy uProxy = new ERC1967Proxy(UImplementation, uTokenInitializeData);
        console.log("U Proxy deployed at:", address(uProxy));

        uToken = BaseToken(address(uProxy));
        uToken.mint(owner, 1000 * SCALING_FACTOR);

        string memory targetContract = "union10aauk4w7883v8lyjv8elleaztnctd3cg2j5a6x6u53p99px0ntyq85zrkl";
        string memory solverAddress = "union1uuuuuuuuu9un2qpksam7rlttpxc8dc76mcphhsmp39pxjnsvrtcqvyv57r";

        uint32 unionChannelId = 1;

        string memory baseTokenSymbol = "U";
        string memory baseTokenName = "au";
        bytes memory quoteToken = abi.encodePacked(uint16(0x6175));
        address implementation = address(new Lst());

        console.log("implementation deployed at:", implementation);

        address feeReceiver = 0x5e9db1aE296D7Bb3F67090964c621E626696F604;
        uint256 feeRate = (1 * SCALING_FACTOR) / 100;
        uint256 minStake = 100000;
        uint256 minUnstake = 100000;

        InitializePayload memory payload = InitializePayload({
            owner: owner,
            lsToken: address(eu),
            zkgm: ucs03,
            unionLstContractAddress: targetContract,
            unionSolverAddress: solverAddress,
            unionChannelId: unionChannelId,
            baseToken: address(uToken),
            baseTokenSymbol: baseTokenSymbol,
            baseTokenName: baseTokenName,
            quoteToken: quoteToken,
            feeReceiver: feeReceiver,
            feeRate: feeRate,
            hubBatchPeriod: 300,
            unbondingBatchPeriod: 600,
            minStake: minStake,
            minUnstake: minUnstake
        });

        bytes memory initializeData = abi.encodeWithSelector(Lst.initialize.selector, payload);

        lstProxy = new LSTProxy(implementation, initializeData);
        console.log("Proxy deployed at:", address(lstProxy));

        lst = Lst(address(lstProxy));
        console.log("Version:", lst.getVersion());

        assertEq(eu.owner(), owner);
        assertEq(lst.owner(), owner);
        eu.transferOwnership(address(lst));
        lst.acceptOwnershipTransfer();
    }

    function test_StakeUnstake() public {
        uint256 amount = 10000000;
        bytes memory recipient = abi.encodePacked(address(owner));
        uint32 recipientChannelId = 0;

        uint256 euBalance1 = eu.balanceOf(owner);
        assertEq(0, euBalance1);
        // allow the LSToken contract to spend the tokens
        uToken.approve(address(lst), amount);
        uint256 recordId = lst.stake(amount, recipient, recipientChannelId);
        uint256 euBalance2 = eu.balanceOf(owner);
        assertEq(amount, euBalance2);
        assertEq(1, recordId);

        HubBatch memory batch = lst.getHubBatch(1);
        assertEq(batch.id, 1);
        assertEq(batch.stakeAmount, 10000000);
        assertEq(batch.mintAmount, 10000000);

        amount = 9000000;
        // allow the eu contract to spend the tokens
        uToken.approve(address(lst), amount);
        recordId = lst.stake(amount, recipient, recipientChannelId);
        assertEq(2, recordId);
        HubBatch memory batch2 = lst.getHubBatch(1);
        assertEq(batch2.stakeAmount, 19000000);
        assertEq(batch2.mintAmount, 19000000);

        amount = 1000000;
        // allow the eu contract to spend the tokens
        eu.approve(address(lst), amount);
        recordId = lst.unstake(amount, recipient, recipientChannelId);
        assertEq(3, recordId);
        HubBatch memory batch3 = lst.getHubBatch(1);
        assertEq(batch3.stakeAmount, 19000000);
        assertEq(batch3.unstakeAmount, 1000000);

        amount = 1000000;
        eu.approve(address(lst), amount);
        recordId = lst.unstake(amount, recipient, recipientChannelId);
        assertEq(4, recordId);
        HubBatch memory batch4 = lst.getHubBatch(1);
        assertEq(batch4.stakeAmount, 19000000);
        assertEq(batch4.unstakeAmount, 2000000);
        assertEq(uint256(batch4.status), uint256(BatchStatus.Pending));
        console.log(" ");
        console.log("========== BEFORE SUBMIT BATCH ==========");
        console.log("stakeAmount", batch4.stakeAmount, "unstakeAmount", batch4.unstakeAmount);
        console.log("mintAmount", batch4.mintAmount, "releasedAmount", batch4.releasedAmount);

        assertEq(1, lst.currentHubBatchId());

        // bytes memory rawSalt = abi.encodePacked(block.timestamp, owner);
        // bytes32 salt = keccak256(rawSalt);
        // lst.submitBatch(salt);

        // console.log("========== AFTER SUBMIT BATCH ==========");

        // assertEq(2, lst.currentHubBatchId());

        // batch = lst.getHubBatch(1);
        // console.log("stakeAmount", batch.stakeAmount, "unstakeAmount", batch.unstakeAmount);

        // console.log("mintAmount", batch.mintAmount, "releasedAmount", batch.releasedAmount);
        // assertEq(uint256(batch.status), uint256(BatchStatus.Executed));
    }

    function test_distributionFormula() public view {
        uint256 totalReceived = 12200;
        uint256 totalStake = 11000;
        uint256 userStake1 = 1000;
        uint256 userStake2 = 1500;
        uint256 userStake3 = 3500;
        uint256 userStake4 = 4500;
        uint256 userStake5 = 500;

        uint256 released1 = lst.calculateReleaseAmount(totalReceived, userStake1, totalStake);
        assertEq(1109, released1);

        uint256 released2 = lst.calculateReleaseAmount(totalReceived, userStake2, totalStake);
        assertEq(1663, released2);

        uint256 released3 = lst.calculateReleaseAmount(totalReceived, userStake3, totalStake);
        assertEq(3881, released3);

        uint256 released4 = lst.calculateReleaseAmount(totalReceived, userStake4, totalStake);
        assertEq(4990, released4);

        uint256 released5 = lst.calculateReleaseAmount(totalReceived, userStake5, totalStake);
        assertEq(554, released5);

        uint256 totalReleased = released1 + released2 + released3 + released4 + released5;
        assertEq(12197, totalReleased);
    }

    // function test_decode() public pure {
    //     uint8[192] memory arr = [
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         128,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         2,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         16,
    //         225,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         14,
    //         12,
    //         105,
    //         124,
    //         146,
    //         32,
    //         192,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         5,
    //         115,
    //         116,
    //         97,
    //         107,
    //         101,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0,
    //         0
    //     ];
    //     bytes memory data = new bytes(192);
    //     for (uint i = 0; i < 192; i++) {
    //         data[i] = bytes1(arr[i]);
    //     }
    //     ZkgmMsg memory zkgmMsg = ZkgmLib.decode(data);

    //     assertEq(2, zkgmMsg.id);
    //     assertEq("stake", zkgmMsg.action);
    //     assertEq(4321, zkgmMsg.amount);
    //     assertEq(1012300000000000000, zkgmMsg.rate);
    // }
}
