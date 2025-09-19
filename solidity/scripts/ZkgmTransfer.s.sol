// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import "@common/zkgm-core/Zkgm.sol";
import "union/apps/ucs/03-zkgm/IZkgmable.sol";
import "union/apps/ucs/03-zkgm/IZkgm.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract ZkgmTransfer is Script {
    address zkgm = 0x5FbE74A283f7954f10AA04C2eDf55578811aeb03;
    IERC20 t;

    address sender = 0x1285a2214319Eff512C5035933ac44E573738772;

    function setUp() public {
        t = IERC20(0xeeEEeeE98622c19Ea39Ea8827ae22Bbfc732671c);
    }

    function zkgmGo(uint32 channelId, uint256 amount, bytes memory quoteToken, bytes memory receiver) internal {
        TokenOrderV2 memory tokenOrder = TokenOrderV2({
            sender: abi.encodePacked(sender),
            receiver: receiver,
            baseToken: abi.encodePacked(t),
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
        // if (t.balanceOf(sender) < amount) {
        //     t.mint(sender, amount);
        // }
        if (t.allowance(sender, zkgm) < amount) {
            t.approve(zkgm, amount);
        }
        bytes32 salt = keccak256(abi.encodePacked(sender, t, channelId, timeoutTimestamp));

        IZkgm(zkgm).send(channelId, 0, timeoutTimestamp, salt, instruction);
        vm.stopBroadcast();
    }

    function run(address token, uint256 amount, uint32 channelId, string memory quoteToken, string memory receiver)
        public
    {
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
