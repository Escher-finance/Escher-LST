// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {LiquidStakingManager} from "../src/LiquidStakingManager.sol";
import {DelegationManager} from "../src/DelegationManager.sol";
import {InitializePayload} from "../src/models/Type.sol";
import {Lst} from "../src/tokens/Lst.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract DeployLiquidStakingManager is Script {
    LiquidStakingManager public lstManager;
    Lst public lst;

    function setUp() public {}

    function run() external returns (LiquidStakingManager) {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        address implementation = address(new Lst());

        string memory lstName = "eHype";
        string memory lstSymbol = "eHP";
        bytes memory initializeData = abi.encodeWithSelector(lst.initialize.selector, msg.sender, lstName, lstSymbol);
        ERC1967Proxy proxy = new ERC1967Proxy(implementation, initializeData);

        lst = Lst(address(proxy));

        console.log("eHype address", address(lst));

        address lstManagerImplementation = address(new LiquidStakingManager());
        console.log("LiquidStakingManager implementation deployed @", lstManagerImplementation);

        InitializePayload memory payload = InitializePayload({initialOwner: msg.sender, lstAddress: address(lst)});

        ERC1967Proxy lstProxy = new ERC1967Proxy(
            lstManagerImplementation, abi.encodeWithSelector(LiquidStakingManager.initialize.selector, payload)
        );
        console.log("LST Manager Proxy deployed @", address(lstProxy));

        lstManager = LiquidStakingManager(payable(address(lstProxy)));

        // transfer Liquid staking token (eU) ownership to Liquid stakign contract
        lst.transferOwnership(address(lstProxy));

        console.log("lstManager address", address(lstManager));

        vm.stopBroadcast();
        return lstManager;
    }
}
