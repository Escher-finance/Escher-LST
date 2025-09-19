// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {eU} from "../src/eU.sol";

contract DeployeUImplementationScript is Script {
    eU public euImplementation;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();
        euImplementation = new eU{salt: 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff}();
        console.log("eU implementation deployed @", address(euImplementation));
        vm.stopBroadcast();
    }
}
