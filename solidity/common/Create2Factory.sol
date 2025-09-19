// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "@openzeppelin/contracts/utils/Create2.sol";

contract Create2Factory {
    event Deployed(address indexed _addr, bytes32 _salt);

    function deploy(bytes32 _salt, bytes memory _creationCode) external returns (address) {
        address addr;
        assembly {
            addr := create2(0, add(_creationCode, 0x20), mload(_creationCode), _salt)
            if iszero(extcodesize(addr)) { revert(0, 0) }
        }
        emit Deployed(addr, _salt);
        return addr;
    }

    function computeAddress(bytes32 _salt, bytes32 _creationCodeHash) external view returns (address) {
        return Create2.computeAddress(_salt, _creationCodeHash, address(this));
    }
}
