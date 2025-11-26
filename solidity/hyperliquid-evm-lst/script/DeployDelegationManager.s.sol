// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;

import {Script, console} from "forge-std/Script.sol";
import {DelegationManager} from "../src/DelegationManager.sol";
import {DepositToken} from "../src/tokens/DepositToken.sol";
import {L1Read} from "../src/L1Read.sol";

contract DeployDelegationManager is Script {
    function setUp() public {}

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        DelegationManager delegationManager = new DelegationManager();

        console.log("delegationManager address", address(delegationManager));
        vm.stopBroadcast();
    }
}
