// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import "../src/ValidatorSetManager.sol";
import {Validator} from "../src/models/Type.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract ValidatorSetManagerTest is Test {
    ValidatorSetManager public validatorSetManager;
    ValidatorSetManager public implementation;

    address public owner;
    address public manager;
    address public user;
    address public validator1;
    address public validator2;
    address public validator3;

    event ValidatorSetBatchUpdated(uint256 count);

    function setUp() public {
        owner = makeAddr("owner");
        manager = makeAddr("manager");
        user = makeAddr("user");
        validator1 = makeAddr("validator1");
        validator2 = makeAddr("validator2");
        validator3 = makeAddr("validator3");

        // Deploy implementation
        implementation = new ValidatorSetManager();

        // Deploy proxy
        bytes memory initData = abi.encodeWithSelector(ValidatorSetManager.initialize.selector, owner, manager);

        ERC1967Proxy proxy = new ERC1967Proxy(address(implementation), initData);

        validatorSetManager = ValidatorSetManager(address(proxy));

        vm.prank(owner);
        validatorSetManager.setDelegationManager(manager);
    }

    /* ============ Initialization Tests ============ */

    function test_Initialize() public view {
        assertEq(validatorSetManager.owner(), owner);
        assertTrue(validatorSetManager.hasRole(validatorSetManager.MANAGER_ROLE(), manager));
        assertEq(validatorSetManager.totalWeight(), 0);
        assertEq(validatorSetManager.getValidatorCount(), 0);

        // Check that owner has DEFAULT_ADMIN_ROLE
        bytes32 defaultAdminRole = validatorSetManager.DEFAULT_ADMIN_ROLE();
        assertTrue(validatorSetManager.hasRole(defaultAdminRole, owner), "Owner should have DEFAULT_ADMIN_ROLE");
    }

    function test_Initialize_RevertsOnZeroAddress() public {
        ValidatorSetManager newImpl = new ValidatorSetManager();

        bytes memory initData = abi.encodeWithSelector(ValidatorSetManager.initialize.selector, address(0), manager);

        vm.expectRevert(IValidatorSetManager.InvalidAddress.selector);
        new ERC1967Proxy(address(newImpl), initData);
    }

    function test_Initialize_CannotReinitialize() public {
        vm.expectRevert();
        validatorSetManager.initialize(owner);
    }

    /* ============ Update Validators Tests ============ */

    function test_UpdateValidators_Success() public {
        address[] memory validators = new address[](3);
        validators[0] = validator1;
        validators[1] = validator2;
        validators[2] = validator3;

        uint64[] memory weights = new uint64[](3);
        weights[0] = 100;
        weights[1] = 200;
        weights[2] = 300;

        vm.prank(manager);
        vm.expectEmit(true, true, true, true);
        emit ValidatorSetBatchUpdated(3);
        validatorSetManager.updateValidators(validators, weights);

        assertEq(validatorSetManager.getValidatorCount(), 3);
        assertEq(validatorSetManager.totalWeight(), 600);
        assertTrue(validatorSetManager.validatorExists(validator1));
        assertTrue(validatorSetManager.validatorExists(validator2));
        assertTrue(validatorSetManager.validatorExists(validator3));
    }

    function test_UpdateValidators_ReplaceExisting() public {
        // First update
        address[] memory validators1 = new address[](2);
        validators1[0] = validator1;
        validators1[1] = validator2;

        uint64[] memory weights1 = new uint64[](2);
        weights1[0] = 100;
        weights1[1] = 200;

        vm.prank(manager);
        validatorSetManager.updateValidators(validators1, weights1);

        // Second update - replace with new set
        address[] memory validators2 = new address[](2);
        validators2[0] = validator2;
        validators2[1] = validator3;

        uint64[] memory weights2 = new uint64[](2);
        weights2[0] = 150;
        weights2[1] = 250;

        vm.prank(manager);
        validatorSetManager.updateValidators(validators2, weights2);

        assertEq(validatorSetManager.getValidatorCount(), 2);
        assertEq(validatorSetManager.totalWeight(), 400);
        assertFalse(validatorSetManager.validatorExists(validator1));
        assertTrue(validatorSetManager.validatorExists(validator2));
        assertTrue(validatorSetManager.validatorExists(validator3));
    }

    function test_UpdateValidators_RevertsOnEmptyArray() public {
        address[] memory validators = new address[](0);
        uint64[] memory weights = new uint64[](0);

        vm.prank(manager);
        vm.expectRevert("Requires minimum 1 validator");
        validatorSetManager.updateValidators(validators, weights);
    }

    function test_UpdateValidators_RevertsOnArrayLengthMismatch() public {
        address[] memory validators = new address[](3);
        validators[0] = validator1;
        validators[1] = validator2;
        validators[2] = validator3;

        uint64[] memory weights = new uint64[](2);
        weights[0] = 100;
        weights[1] = 200;

        vm.prank(manager);
        vm.expectRevert(IValidatorSetManager.ArrayLengthMismatch.selector);
        validatorSetManager.updateValidators(validators, weights);
    }

    function test_UpdateValidators_RevertsOnZeroAddress() public {
        address[] memory validators = new address[](2);
        validators[0] = validator1;
        validators[1] = address(0);

        uint64[] memory weights = new uint64[](2);
        weights[0] = 100;
        weights[1] = 200;

        vm.prank(manager);
        vm.expectRevert(IValidatorSetManager.InvalidAddress.selector);
        validatorSetManager.updateValidators(validators, weights);
    }

    function test_UpdateValidators_RevertsOnZeroWeight() public {
        address[] memory validators = new address[](2);
        validators[0] = validator1;
        validators[1] = validator2;

        uint64[] memory weights = new uint64[](2);
        weights[0] = 100;
        weights[1] = 0;

        vm.prank(manager);
        vm.expectRevert(IValidatorSetManager.InvalidWeight.selector);
        validatorSetManager.updateValidators(validators, weights);
    }

    function test_UpdateValidators_RevertsOnNonManager() public {
        address[] memory validators = new address[](1);
        validators[0] = validator1;

        uint64[] memory weights = new uint64[](1);
        weights[0] = 100;

        vm.prank(user);
        vm.expectRevert("Caller is not a manager");
        validatorSetManager.updateValidators(validators, weights);
    }

    /* ============ Getter Tests ============ */

    function test_GetValidator() public {
        address[] memory validators = new address[](2);
        validators[0] = validator1;
        validators[1] = validator2;

        uint64[] memory weights = new uint64[](2);
        weights[0] = 100;
        weights[1] = 200;

        vm.prank(manager);
        validatorSetManager.updateValidators(validators, weights);

        (address addr, uint64 weight) = validatorSetManager.getValidator(validator1);
        assertEq(addr, validator1);
        assertEq(weight, 100);

        (addr, weight) = validatorSetManager.getValidator(validator2);
        assertEq(addr, validator2);
        assertEq(weight, 200);
    }

    function test_GetValidator_RevertsOnNotFound() public {
        vm.expectRevert("Validator not found");
        validatorSetManager.getValidator(validator1);
    }

    function test_GetAllValidators() public {
        address[] memory validators = new address[](3);
        validators[0] = validator1;
        validators[1] = validator2;
        validators[2] = validator3;

        uint64[] memory weights = new uint64[](3);
        weights[0] = 100;
        weights[1] = 200;
        weights[2] = 300;

        vm.prank(manager);
        validatorSetManager.updateValidators(validators, weights);

        Validator[] memory allValidators = validatorSetManager.getAllValidators();
        assertEq(allValidators.length, 3);
        assertEq(allValidators[0].validator, validator1);
        assertEq(allValidators[0].weight, 100);
        assertEq(allValidators[1].validator, validator2);
        assertEq(allValidators[1].weight, 200);
        assertEq(allValidators[2].validator, validator3);
        assertEq(allValidators[2].weight, 300);
    }

    function test_ValidatorExists() public {
        address[] memory validators = new address[](1);
        validators[0] = validator1;

        uint64[] memory weights = new uint64[](1);
        weights[0] = 100;

        vm.prank(manager);
        validatorSetManager.updateValidators(validators, weights);

        assertTrue(validatorSetManager.validatorExists(validator1));
        assertFalse(validatorSetManager.validatorExists(validator2));
    }

    /* ============ Access Control Tests ============ */

    function test_OwnerCanGrantManagerRole() public {
        address newManager = makeAddr("newManager");

        bytes32 managerRole = validatorSetManager.MANAGER_ROLE();
        vm.prank(owner);
        validatorSetManager.grantRole(managerRole, newManager);

        assertTrue(validatorSetManager.hasRole(managerRole, newManager));
    }

    function test_OwnerCanRevokeManagerRole() public {
        address theOwner = validatorSetManager.owner();

        bytes32 managerRole = validatorSetManager.MANAGER_ROLE();
        vm.prank(owner);
        validatorSetManager.revokeRole(managerRole, manager);
        assertFalse(validatorSetManager.hasRole(managerRole, manager));
    }

    function test_NonOwnerCannotGrantManagerRole() public {
        address newManager = makeAddr("newManager");

        bytes32 managerRole = validatorSetManager.MANAGER_ROLE();
        vm.prank(user);
        vm.expectRevert();
        validatorSetManager.grantRole(managerRole, newManager);
    }

    /* ============ UUPS Upgrade Tests ============ */

    function test_OwnerCanUpgrade() public {
        ValidatorSetManager newImpl = new ValidatorSetManager();

        vm.prank(owner);
        validatorSetManager.upgradeToAndCall(address(newImpl), "");
    }

    function test_NonOwnerCannotUpgrade() public {
        ValidatorSetManager newImpl = new ValidatorSetManager();

        vm.prank(user);
        vm.expectRevert();
        validatorSetManager.upgradeToAndCall(address(newImpl), "");
    }

    /* ============ Fuzz Tests ============ */

    function testFuzz_UpdateValidators(uint8 validatorCount, uint64 baseWeight) public {
        vm.assume(validatorCount > 0 && validatorCount <= 50);
        vm.assume(baseWeight > 0 && baseWeight < type(uint64).max / validatorCount);

        address[] memory validators = new address[](validatorCount);
        uint64[] memory weights = new uint64[](validatorCount);
        uint64 expectedTotal = 0;

        for (uint256 i = 0; i < validatorCount; i++) {
            validators[i] = makeAddr(string(abi.encodePacked("validator", i)));
            weights[i] = baseWeight + uint64(i);
            expectedTotal += weights[i];
        }

        vm.prank(manager);
        validatorSetManager.updateValidators(validators, weights);

        assertEq(validatorSetManager.getValidatorCount(), validatorCount);
        assertEq(validatorSetManager.totalWeight(), expectedTotal);
    }

    /* ============ Edge Case Tests ============ */

    function test_UpdateValidators_MaxWeight() public {
        address[] memory validators = new address[](1);
        validators[0] = validator1;

        uint64[] memory weights = new uint64[](1);
        weights[0] = type(uint64).max;

        vm.prank(manager);
        validatorSetManager.updateValidators(validators, weights);

        assertEq(validatorSetManager.totalWeight(), type(uint64).max);
    }

    function test_UpdateValidators_MultipleUpdates() public {
        // First update
        address[] memory validators1 = new address[](1);
        validators1[0] = validator1;
        uint64[] memory weights1 = new uint64[](1);
        weights1[0] = 100;

        vm.prank(manager);
        validatorSetManager.updateValidators(validators1, weights1);

        // Second update
        address[] memory validators2 = new address[](1);
        validators2[0] = validator2;
        uint64[] memory weights2 = new uint64[](1);
        weights2[0] = 200;

        vm.prank(manager);
        validatorSetManager.updateValidators(validators2, weights2);

        // Third update
        address[] memory validators3 = new address[](1);
        validators3[0] = validator3;
        uint64[] memory weights3 = new uint64[](1);
        weights3[0] = 300;

        vm.prank(manager);
        validatorSetManager.updateValidators(validators3, weights3);

        assertEq(validatorSetManager.getValidatorCount(), 1);
        assertEq(validatorSetManager.totalWeight(), 300);
        assertFalse(validatorSetManager.validatorExists(validator1));
        assertFalse(validatorSetManager.validatorExists(validator2));
        assertTrue(validatorSetManager.validatorExists(validator3));
    }
}
