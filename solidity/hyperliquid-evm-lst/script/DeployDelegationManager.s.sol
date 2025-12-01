// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {DelegationManager} from "../src/DelegationManager.sol";
import {ValidatorSetManager} from "../src/ValidatorSetManager.sol";
import {L1Read} from "../src/L1Read.sol";

contract DeployDelegationManager is Script {
    function setUp() public {}

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        ValidatorSetManager validatorManager = new ValidatorSetManager();

        DelegationManager delegationManager = new DelegationManager();
        delegationManager.initialize(msg.sender, address(validatorManager));

        console.log("delegationManager address", address(delegationManager));
        vm.stopBroadcast();
    }
}
