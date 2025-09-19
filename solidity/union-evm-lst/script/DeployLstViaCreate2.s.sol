// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Lst, InitializePayload} from "../src/Lst.sol";
import {LSTProxy} from "../src/LSTProxy.sol";
import {Create2Factory} from "@common/Create2Factory.sol";
import {Upgrades} from "openzeppelin-foundry-upgrades/Upgrades.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract DeployLstViaCreate2Script is Script {
    Lst public lst;

    function setUp() public {}

    function run() public {
        address uToken = 0xba5eD44733953d79717F6269357C77718C8Ba5ed;
        address ucs03 = 0x5FbE74A283f7954f10AA04C2eDf55578811aeb03;

        string memory lstContract = "union17auke0f2l9uejk6wfkwvhlap3fxdqcdl8jkcmg6cmc53cnqrhekq7928m0";
        string memory solverAddress = "union1uuuuuuuuu9un2qpksam7rlttpxc8dc76mcphhsmp39pxjnsvrtcqvyv57r";
        uint32 unionChannelId = 5;
        address baseToken = 0xba5eD44733953d79717F6269357C77718C8Ba5ed;
        string memory baseTokenSymbol = "U";
        string memory baseTokenName = "au";
        bytes memory quoteToken = abi.encodePacked("au");
        address feeReceiver = 0x15Ee7c367F4232241028c36E720803100757c6e9;
        uint256 feeRate = 0;
        uint256 minStake = 100000;
        uint256 minUnstake = 100000;

        InitializePayload memory payload = InitializePayload({
            owner: msg.sender,
            lsToken: uToken,
            zkgm: ucs03,
            unionLstContractAddress: lstContract,
            unionSolverAddress: solverAddress,
            unionChannelId: unionChannelId,
            baseToken: baseToken,
            baseTokenSymbol: baseTokenSymbol,
            baseTokenName: baseTokenName,
            quoteToken: quoteToken,
            feeReceiver: feeReceiver,
            feeRate: feeRate,
            hubBatchPeriod: 300,
            unbondingBatchPeriod: 600,
            minStake: minStake,
            minUnstake: minUnstake
        });

        address lstImplementation = 0x14DF5C804533BB4D8eC24b584a5f9e44bAdB96BD;
        Create2Factory factory = Create2Factory(0x226143977e08FEA768e5f11f37DCE22f9dF8be33);
        console.log("using lstImplementation:", lstImplementation);

        bytes memory data = abi.encodeWithSelector(Lst.initialize.selector, payload);

        bytes memory initcode = abi.encodePacked(type(LSTProxy).creationCode, abi.encode(lstImplementation, data));

        bytes32 hash = keccak256(initcode);
        console.log("hash:");
        console.logBytes32(hash);

        bytes32 salt = 0x0000000000000000000000000000000000000000000000000000000000000000;
        console.log("using salt:");
        console.logBytes32(salt);

        vm.startBroadcast();

        address lstProxy = factory.deploy(salt, initcode);

        console.log("LSTProxy deployed @", lstProxy);

        lst = Lst(lstProxy);
        console.log("Version:", lst.getVersion());

        vm.stopBroadcast();
    }
}
