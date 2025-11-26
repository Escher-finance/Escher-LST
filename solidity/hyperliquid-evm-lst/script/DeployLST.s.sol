// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;

import {Script, console} from "forge-std/Script.sol";
import {LiquidStakingManager} from "../src/LiquidStakingManager.sol";
import {DepositToken} from "../src/tokens/DepositToken.sol";
import {DelegationManager} from "../src/DelegationManager.sol";

contract DeployLiquidStakingManager is Script {
    LiquidStakingManager public lstManager;

    function setUp() public {}

    function run() external returns (LiquidStakingManager) {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);
        address deployerAddress = vm.addr(deployerPrivateKey);

        DepositToken asset = new DepositToken("USDC", "usdc");

        lstManager = new LiquidStakingManager(deployerAddress, address(asset), "eToken", "token");

        vm.stopBroadcast();
        return lstManager;
    }
}
