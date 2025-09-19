// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Lst} from "../src/Lst.sol";
import {BaseToken} from "../src/U.sol";
import {HubBatch, HubRecord} from "@common/zkgm-core/Types.sol";

contract RunSubmitBatch is Script {
    Lst public lst;
    address evmHub = 0xa721272Ea5328a60806dEcd1C6396d49E2bB4583;

    function run() public {
        vm.startBroadcast();

        lst = Lst(evmHub);

        uint32 currentHubBatchId = lst.currentHubBatchId();
        console.log("current hub batch id", currentHubBatchId);
        console.log("lastHubBatchTimestamp", lst.lastHubBatchTimestamp());
        HubBatch memory batch = lst.getHubBatch(currentHubBatchId);
        console.log("batch stake Amount", batch.stakeAmount);
        console.log("batch status ", uint256(batch.status));
        bytes memory rawSalt = abi.encodePacked(block.timestamp, msg.sender);
        bytes32 salt = keccak256(rawSalt);
        lst.submitBatch(salt);

        console.log("after submit hub batch id", lst.currentHubBatchId());
        HubBatch memory batch2 = lst.getHubBatch(currentHubBatchId);
        console.log("hub status after submit", uint256(batch2.status));
        console.log("hub status stakeAmount", batch2.stakeAmount);
        console.log("hub status unstakeAmount", batch2.unstakeAmount);
        console.log("hub status mintAmount", batch2.mintAmount);
        console.log("hub status releasedAmount", batch2.releasedAmount);

        vm.stopBroadcast();
    }
}
