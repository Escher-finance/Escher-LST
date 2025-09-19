// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {Create2Factory} from "../src/Create2Factory.sol";
import {LSTProxy} from "../src/LSTProxy.sol";
import {Lst, InitializePayload} from "../src/Lst.sol";

contract LSTProxyFactoryTest is Test {
    Create2Factory factory;
    bytes initcode;

    function setUp() public {
        factory = new Create2Factory();

        Lst implementation = new Lst();
        uint256 minStake = 100000;
        uint256 minUnstake = 100000;

        InitializePayload memory payload = InitializePayload({
            owner: makeAddr("owner"),
            lsToken: makeAddr("lsToken"),
            zkgm: makeAddr("zkgm"),
            unionLstContractAddress: "a",
            unionSolverAddress: "a",
            unionChannelId: 0,
            baseToken: makeAddr("baseToken"),
            baseTokenSymbol: "a",
            baseTokenName: "a",
            quoteToken: hex"ffffff",
            feeReceiver: makeAddr("fee"),
            feeRate: 0,
            hubBatchPeriod: 300,
            unbondingBatchPeriod: 600,
            minStake: minStake,
            minUnstake: minUnstake
        });
        bytes memory data = abi.encodeWithSelector(implementation.initialize.selector, payload);
        initcode = abi.encodePacked(type(LSTProxy).creationCode, abi.encode(address(implementation), data));
    }

    function test_create2() public {
        bytes32 salt = keccak256("escherrr");
        address computedProxy = factory.computeAddress(salt, keccak256(initcode));
        address proxy = factory.deploy(salt, initcode);
        assertEq(computedProxy, proxy);
    }
}
