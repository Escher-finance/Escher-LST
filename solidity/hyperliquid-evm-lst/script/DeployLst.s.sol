// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {DelegationManager} from "../src/DelegationManager.sol";
import {Lst} from "../src/tokens/Lst.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract DeployLst is Script {
    Lst public lst;

    function setUp() public {}

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        address implementation = address(new Lst());

        string memory lstName = "eHype";
        string memory lstSymbol = "eHP";
        bytes memory initializeData = abi.encodeWithSelector(lst.initialize.selector, msg.sender, lstName, lstSymbol);
        ERC1967Proxy proxy = new ERC1967Proxy(implementation, initializeData);

        lst = Lst(address(proxy));

        console.log("lst address", address(lst));
        vm.stopBroadcast();
    }
}
