// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Lst} from "../src/Lst.sol";

contract DeployLstImplementationScript is Script {
    Lst public lstImplementation;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();
        lstImplementation = new Lst();
        console.log("Lst implementation deployed @", address(lstImplementation));
        vm.stopBroadcast();
    }
}
