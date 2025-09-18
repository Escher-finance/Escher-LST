// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {eU} from "../src/eU.sol";
import {Create2Factory} from "../src/Create2Factory.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract DeployeUScript is Script {
    eU public eu;
    address zkgm = 0x5FbE74A283f7954f10AA04C2eDf55578811aeb03;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        address implementation = address(0x0f6f7A2846d760C334369918a083940feA3601Cb);

        Create2Factory factory = Create2Factory(0x226143977e08FEA768e5f11f37DCE22f9dF8be33);

        address owner = 0x1285a2214319Eff512C5035933ac44E573738772;
        bytes memory initializeData = abi.encodeWithSelector(eu.initialize.selector, owner, zkgm);

        bytes memory initcode =
            abi.encodePacked(type(ERC1967Proxy).creationCode, abi.encode(implementation, initializeData));
        bytes32 initcodeHash = keccak256(initcode);

        console.log("sender", msg.sender);
        console.log("hash");
        console.logBytes32(initcodeHash);

        bytes32 salt = 0x365f317b09d71d84fc1a4a354fb23a063adf1c28ee2b9581ef51ae27514c15c0;

        address proxy = factory.deploy(salt, initcode);

        console.log("eU:", address(proxy));

        vm.stopBroadcast();
    }
}
