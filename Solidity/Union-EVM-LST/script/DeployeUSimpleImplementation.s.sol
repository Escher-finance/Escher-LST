// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {eUSimple} from "../src/eUSimple.sol";

contract DeployeUSimpleImplementationScript is Script {
    eUSimple public euSimpleImplementation;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();
        euSimpleImplementation =
            new eUSimple{salt: 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff}();
        console.log("eUSimple implementation deployed @", address(euSimpleImplementation));
        vm.stopBroadcast();
    }
}
