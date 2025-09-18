// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Lst} from "../src/Lst.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract RunTransferAndCall is Script {
    Lst public lst;
    IERC20 public uToken;

    function run() public {
        // vm.startBroadcast();
        // address evmHub = 0x8369f166f77CE6656ceBFF134Dd17F1e0e71eE46; // Replace with actual evm contract address
        //
        // uToken = IERC20(0xba5eD44733953d79717F6269357C77718C8Ba5ed);
        // uint256 amount = 10000; // Amount to transfer
        // uint256 balance = uToken.balanceOf(msg.sender);
        // console.log("Balance of sender:", msg.sender, balance);
        //
        // uToken.approve(evmHub, amount);
        //
        // lst = Lst(evmHub);
        // bytes memory rawSalt = abi.encodePacked(block.timestamp, msg.sender);
        // bytes32 salt = keccak256(rawSalt);
        // console.log("salt", vm.toString(salt));
        //
        // lst.transferAndCall(amount, salt);
        //
        // vm.stopBroadcast();
    }
}
