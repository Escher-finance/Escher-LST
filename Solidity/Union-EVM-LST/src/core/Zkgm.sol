// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "union/apps/ucs/03-zkgm/Lib.sol";

library ZkgmLstLib {
    function makeCall(address sender, bool eureka, bytes memory contractAddress, bytes memory contractCalldata)
        internal
        pure
        returns (Instruction memory)
    {
        return Instruction({
            version: ZkgmLib.INSTR_VERSION_0,
            opcode: ZkgmLib.OP_CALL,
            operand: ZkgmLib.encodeCall(
                Call({
                    sender: abi.encodePacked(sender),
                    eureka: eureka,
                    contractAddress: contractAddress,
                    contractCalldata: contractCalldata
                })
            )
        });
    }

    function decode(bytes calldata stream) external pure returns (ZkgmMsg calldata) {
        ZkgmMsg calldata operand;
        assembly {
            operand := stream.offset
        }
        return operand;
    }
}

struct ZkgmMsg {
    string action;
    uint32 id;
    uint256 amount;
    uint256 rate;
    uint64 union_block; // Add this field for union block information
}
