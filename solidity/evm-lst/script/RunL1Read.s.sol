// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {LiquidStakingManager} from "../src/LiquidStakingManager.sol";
import {L1Read} from "../src/L1Read.sol";

contract RunL1Read is Script {
    function setUp() public {}

    function run() public view {
        uint256 currentChainId = block.chainid;
        console.log("Current Chain ID:", currentChainId);
        L1Read l1Read = L1Read(0xb0FBCF71E600383C1298413BFDEFc4A32240B033);
        address user = 0x15Ee7c367F4232241028c36E720803100757c6e9;
        L1Read.CoreUserExists memory coreUser = l1Read.coreUserExists(user);
        console.log("coreUser.exists:", coreUser.exists);
    }
}
