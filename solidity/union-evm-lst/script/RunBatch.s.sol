// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Lst} from "../src/Lst.sol";

contract RunBatch is Script {
    Lst public lst;

    function run() public {
        vm.startBroadcast();
        address evmHub = 0xa721272Ea5328a60806dEcd1C6396d49E2bB4583;
        lst = Lst(evmHub);

        console.log(uint256(lst.getHubBatch(2).status));
        console.log(lst.lastZkgmTimestamp());
        console.log(lst.lastUpdateTimestamp());
        //lst.submitTest("zxywvut");
        vm.stopBroadcast();
    }
}
