// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.28;

import {Script, console} from "forge-std/Script.sol";

interface IBankPrecompile {
    function balanceOf(address account) external view returns (uint256);
}

contract RunStable is Script {
    address GUSDT = 0x0000000000000000000000000000000000001000;

    function setUp() public {}

    function run() public view {
        IBankPrecompile bank = IBankPrecompile(GUSDT);
        uint256 balance = bank.balanceOf(0x15Ee7c367F4232241028c36E720803100757c6e9);
        console.log("balance", balance);
    }
}
