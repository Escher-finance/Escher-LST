// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import "@common/zkgm-core/Zkgm.sol";
import {eU} from "../union-evm-lst/src/eU.sol";
import "union/apps/ucs/03-zkgm/IZkgmable.sol";
import "union/apps/ucs/03-zkgm/IZkgm.sol";

contract RunZkgmSend is Script {
    address zkgm = 0x5FbE74A283f7954f10AA04C2eDf55578811aeb03;
    eU eu;

    address sender = 0x1285a2214319Eff512C5035933ac44E573738772;

    function setUp() public {
        eu = eU(0xeeEEeeE98622c19Ea39Ea8827ae22Bbfc732671c);
    }

    function zkgmGo(uint32 channelId, uint256 amount, bytes memory quoteToken, bytes memory receiver) internal {
        TokenOrderV2 memory tokenOrder = TokenOrderV2({
            sender: abi.encodePacked(sender),
            receiver: receiver,
            baseToken: abi.encodePacked(eu),
            baseAmount: amount,
            quoteToken: quoteToken,
            quoteAmount: amount,
            kind: ZkgmLib.TOKEN_ORDER_KIND_SOLVE,
            metadata: ZkgmLib.encodeSolverMetadata(SolverMetadata({solverAddress: quoteToken, metadata: hex""}))
        });

        Instruction memory instruction = Instruction({
            version: ZkgmLib.INSTR_VERSION_2,
            opcode: ZkgmLib.OP_TOKEN_ORDER,
            operand: ZkgmLib.encodeTokenOrderV2(tokenOrder)
        });

        uint64 timeoutTimestamp = (uint64(block.timestamp) + 3 days) * 1_000_000_000;

        vm.startBroadcast();
        if (eu.balanceOf(sender) < amount) {
            eu.mint(sender, amount);
        }
        if (eu.allowance(sender, zkgm) < amount) {
            eu.approve(zkgm, amount);
        }
        bytes32 salt = keccak256(abi.encodePacked(sender, eu, channelId, timeoutTimestamp));

        IZkgm(zkgm).send(channelId, 0, timeoutTimestamp, salt, instruction);
        vm.stopBroadcast();
    }

    function run() public {
        // holesky > sepolia
        // zkgmGo(1, 1 ether, abi.encodePacked(eu), abi.encodePacked(sender));

        // sepolia > holesky
        // zkgmGo(6, 0.15 ether, abi.encodePacked(eu), abi.encodePacked(sender));

        // holesky > union
        zkgmGo(
            6,
            0.3 ether,
            hex"756e696f6e316e6b6a787374396535666a336464386d7a6d34737664306d676e396d6d37633776736d387a6a70633975736d646a636138787073646577327671",
            hex"756e696f6e31793375346d77333961646e67656e6c7a77716d36687a3630666c7a3235677378327271756861"
        );
    }
}
