// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import "../ffi/FFI.sol";

contract FFITest is Test {
    function setUp() public {}

    function test_ToHex() public {
        string memory utf8Input = "union1y3u4mw39adngenlzwqm6hz60flz25gsx2rquha";
        string memory hexInput =
            "0x756e696f6e31793375346d77333961646e67656e6c7a77716d36687a3630666c7a3235677378327271756861";
        bytes memory target =
            hex"756e696f6e31793375346d77333961646e67656e6c7a77716d36687a3630666c7a3235677378327271756861";

        assertEq(FFIHelper.ffiToHex(vm, utf8Input), target);
        assertEq(FFIHelper.ffiToHex(vm, hexInput), target);
    }
}
