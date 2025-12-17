// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {LiquidStakingManager} from "../src/LiquidStakingManager.sol";
import {HyperliquidDelegationManager} from "../src/contracts/HyperliquidDelegationManager.sol";
import {InitializePayload} from "../src/models/Type.sol";
import {Lst} from "../src/tokens/Lst.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract DeployLiquidStakingManager is Script {
    LiquidStakingManager public lstManager;
    Lst public lst;
    HyperliquidDelegationManager public delegationManager;

    function setUp() public {}

    function run() external returns (LiquidStakingManager) {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        address delegationManagerAddress = 0xD076481EF09255d243C58C65c119C377009Fda31; //TODO: Replace This
        delegationManager = HyperliquidDelegationManager(delegationManagerAddress);

        // Deploying Lst (ERC20)
        address implementation = address(new Lst());

        string memory lstName = "eHype";
        string memory lstSymbol = "eHP";
        bytes memory initializeData = abi.encodeWithSelector(lst.initialize.selector, msg.sender, lstName, lstSymbol);
        ERC1967Proxy proxy = new ERC1967Proxy(implementation, initializeData);

        lst = Lst(address(proxy));

        console.log("eHype address", address(lst));

        // Deploying Lst Manager
        address lstManagerImplementation = address(new LiquidStakingManager());
        console.log("LiquidStakingManager implementation deployed @", lstManagerImplementation);

        ERC1967Proxy lstProxy = new ERC1967Proxy(
            lstManagerImplementation,
            abi.encodeWithSelector(
                LiquidStakingManager.initialize.selector, msg.sender, address(lst), delegationManagerAddress
            )
        );
        console.log("LST Manager Proxy deployed @", address(lstProxy));

        lstManager = LiquidStakingManager(payable(address(lstProxy)));

        delegationManager.setLiquidStakingManager(address(lstManager));

        // transfer Liquid staking token (eU) ownership to Liquid staking contract
        lst.transferOwnership(address(lstProxy));

        console.log("lstManager address", address(lstManager));

        lstManager.acceptOwnershipTransfer();

        vm.stopBroadcast();
        return lstManager;
    }
}
