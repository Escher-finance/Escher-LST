// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Lst} from "../src/Lst.sol";
import {eU} from "../src/eU.sol";
import {BaseToken} from "../src/U.sol";
import {HubBatch, HubRecord} from "@common/zkgm-core/Types.sol";

contract RunStake is Script {
    Lst public lst;
    address baseToken = 0xba5eD44733953d79717F6269357C77718C8Ba5ed;
    address eUTokenAddr = 0x25002bCaC93dA34881fEbC4b1999EAd6c08Cfc27; // eU (Liquid staking token) address
    address evmHub = 0xa721272Ea5328a60806dEcd1C6396d49E2bB4583;

    function run() public {
        vm.startBroadcast();

        lst = Lst(evmHub);

        eU eUToken = eU(eUTokenAddr);

        BaseToken uToken = BaseToken(baseToken);
        console.log("balance U", uToken.balanceOf(msg.sender));
        uint256 euBalance = eUToken.balanceOf(msg.sender);
        console.log("balance eU", euBalance);

        bytes memory sender = abi.encodePacked(msg.sender);

        console.log(eUTokenAddr);

        uint256 amount = 50000;
        uToken.approve(evmHub, amount);

        console.log("current hub batch id", lst.currentHubBatchId());

        uint32 hubBatchId = lst.currentHubBatchId();

        HubBatch memory batch = lst.getHubBatch(hubBatchId);
        console.log("hub status", uint256(batch.status));
        console.log("hub stakeAmount", batch.stakeAmount);

        // lst.stake(amount, sender, 0);

        // uToken.approve(evmHub, 20000);
        // lst.stake(20000, sender, 0);

        // uToken.approve(evmHub, 30000);
        // lst.stake(30000, sender, 0);

        eUToken.approve(evmHub, 10000);
        lst.unstake(10000, sender, 0);

        eUToken.approve(evmHub, 10000);
        lst.unstake(10000, sender, 0);

        HubBatch memory thebatch = lst.getHubBatch(hubBatchId);
        console.log("Total stake", thebatch.stakeAmount);

        console.log("==== AFTER STAKE & UNSTAKE =====");
        uint256 uBalance1 = uToken.balanceOf(msg.sender);
        console.log("balance U", uBalance1);

        uint256 euBalance1 = eUToken.balanceOf(msg.sender);
        console.log("balance eU", euBalance1);

        HubBatch memory batch2 = lst.getHubBatch(hubBatchId);
        console.log("hub batch stakeAmount", batch2.stakeAmount);
        console.log("hub batch unstakeAmount", batch2.unstakeAmount);
        console.log("hub batch releasedAmount", batch2.releasedAmount);
        console.log("hub batch mintAmount", batch2.mintAmount);
        vm.stopBroadcast();
    }
}
