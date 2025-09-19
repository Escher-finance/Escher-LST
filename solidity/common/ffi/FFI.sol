// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Vm.sol";

library FFIHelper {
    function ffiToHex(Vm vm, string memory input) internal returns (bytes memory output) {
        string[] memory cmds = new string[](2);
        cmds[0] = "solidity/common/ffi/to-hex.sh";
        cmds[1] = input;
        output = vm.ffi(cmds);
    }
}
