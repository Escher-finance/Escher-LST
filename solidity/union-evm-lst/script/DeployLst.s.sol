// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Lst, InitializePayload} from "../src/Lst.sol";
import {eU} from "../src/eU.sol";
import {LSTProxy} from "../src/LSTProxy.sol";
import {Upgrades} from "openzeppelin-foundry-upgrades/Upgrades.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract DeployLstScript is Script {
    Lst public lst;
    eU public eu;
    address baseToken = 0xba5eD44733953d79717F6269357C77718C8Ba5ed;
    address ucs03 = 0x5FbE74A283f7954f10AA04C2eDf55578811aeb03;
    string lstContract = "union17auke0f2l9uejk6wfkwvhlap3fxdqcdl8jkcmg6cmc53cnqrhekq7928m0";
    string solverAddress = "union1uuuuuuuuu9un2qpksam7rlttpxc8dc76mcphhsmp39pxjnsvrtcqvyv57r";

    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        address implementation = address(new eU());
        console.log("eU implementation deployed at:", implementation);
        bytes memory initializeData = abi.encodeWithSelector(eu.initialize.selector, msg.sender, ucs03);
        ERC1967Proxy proxy = new ERC1967Proxy(implementation, initializeData);
        console.log("eU Proxy deployed at:", address(proxy));

        eu = eU(address(proxy));
        console.log("eU :", address(eu));

        uint32 unionChannelId = 6;
        uint256 minStake = 100000;
        uint256 minUnstake = 100000;

        InitializePayload memory payload = InitializePayload({
            owner: msg.sender,
            lsToken: address(proxy),
            zkgm: ucs03,
            unionLstContractAddress: lstContract,
            unionSolverAddress: solverAddress,
            unionChannelId: unionChannelId,
            baseToken: baseToken,
            baseTokenSymbol: "U",
            baseTokenName: "au",
            quoteToken: abi.encodePacked("au"),
            feeReceiver: 0x15Ee7c367F4232241028c36E720803100757c6e9,
            feeRate: 0,
            hubBatchPeriod: 60,
            unbondingBatchPeriod: 600,
            minStake: minStake,
            minUnstake: minUnstake
        });

        address lstImplementation = address(new Lst());
        console.log("Lst implementation deployed @", lstImplementation);

        LSTProxy lstProxy = new LSTProxy(lstImplementation, abi.encodeWithSelector(Lst.initialize.selector, payload));
        console.log("LSTProxy deployed @", address(lstProxy));

        // transfer Liquid staking token (eU) ownership to Liquid stakign contract
        eu.transferOwnership(address(lstProxy));

        lst = Lst(address(lstProxy));
        console.log("Version:", lst.getVersion());

        vm.stopBroadcast();
    }
}
