// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Create2Factory} from "../src/Create2Factory.sol";

contract DeployCreate2FactoryScript is Script {
    Create2Factory public factory;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();
        factory = new Create2Factory{salt: 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff}();
        console.log("Create2Factory deployed @", address(factory));
        vm.stopBroadcast();
    }
}
