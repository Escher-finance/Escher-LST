// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {StablechainDelegationManager} from "../src/contracts/StablechainDelegationManager.sol";
import {ValidatorSetManager} from "../src/ValidatorSetManager.sol";
import {L1Read} from "../src/L1Read.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract DeployHyperliquidValidatorDelegationManager is Script {
    ValidatorSetManager validatorManager;
    StablechainDelegationManager delegationManager;

    function setUp() public {}

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);
        // Get the address associated with the private key
        address userAddress = vm.addr(deployerPrivateKey);

        // Start deploy Validator set Manager
        address validatorManagerImpl = address(new ValidatorSetManager());
        bytes memory initializeValidatorManagerData =
            abi.encodeWithSelector(ValidatorSetManager.initialize.selector, msg.sender);
        ERC1967Proxy validatorManagerProxy = new ERC1967Proxy(validatorManagerImpl, initializeValidatorManagerData);
        validatorManager = ValidatorSetManager(address(validatorManagerProxy));
        console.log("validatorManager address", address(validatorManager));

        // Start deploy DelegationManager
        address delegationManagerImpl = address(new StablechainDelegationManager());

        address stableToken = 0x0000000000000000000000000000000000001003;
        address liquidToken = 0x0000000000000000000000000000000000001003;
        bytes memory initializeDelegationManagerData = abi.encodeWithSelector(
            StablechainDelegationManager.initialize.selector,
            msg.sender,
            address(validatorManager),
            stableToken,
            liquidToken
        );

        ERC1967Proxy delegationManagerProxy = new ERC1967Proxy(delegationManagerImpl, initializeDelegationManagerData);
        delegationManager = StablechainDelegationManager(address(delegationManagerProxy));
        console.log("delegationManager address", address(delegationManager));

        validatorManager.setDelegationManager(address(delegationManager));

        delegationManager.setLiquidStakingManager(userAddress);

        vm.stopBroadcast();
    }
}
