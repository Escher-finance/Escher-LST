// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import "@common/zkgm-core/Zkgm.sol";
import "union/apps/ucs/03-zkgm/IZkgmable.sol";
import "union/apps/ucs/03-zkgm/IZkgm.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@common/ffi/FFI.sol";

contract ZkgmTransfer is Script {
    /// @dev Default ZKGM used
    address constant ZKGM = 0x5FbE74A283f7954f10AA04C2eDf55578811aeb03;

    /// @dev Simplest way to run this. ZKGM is the default. Solver is the quote token.
    /// @dev To run this version use `--sig 'run(address,uint256,uint32,string memory,string memory)'`
    function run(address token, uint256 amount, uint32 channelId, string memory quoteToken, string memory receiver)
        public
    {
        IERC20 t = IERC20(token);

        bytes memory hexQuoteToken = FFIHelper.ffiToHex(vm, quoteToken);
        bytes memory hexReceiver = FFIHelper.ffiToHex(vm, receiver);

        _zkgmTokenOrderV2(ZKGM, t, channelId, amount, hexQuoteToken, hexReceiver, hexQuoteToken);
    }

    /// @dev Use custom solver.
    /// @dev To run this version use `--sig 'run(address,uint256,uint32,string memory,string memory,string memory)'`
    function run(
        address token,
        uint256 amount,
        uint32 channelId,
        string memory quoteToken,
        string memory receiver,
        string memory solver
    ) public {
        IERC20 t = IERC20(token);

        bytes memory hexQuoteToken = FFIHelper.ffiToHex(vm, quoteToken);
        bytes memory hexReceiver = FFIHelper.ffiToHex(vm, receiver);
        bytes memory hexSolver = FFIHelper.ffiToHex(vm, solver);

        _zkgmTokenOrderV2(ZKGM, t, channelId, amount, hexQuoteToken, hexReceiver, hexSolver);
    }

    /// @dev Use custom ZKGM and solver.
    /// @dev To run this version use `--sig 'run(address,address,uint256,uint32,string memory,string memory,string memory)'`
    function run(
        address zkgm,
        address token,
        uint256 amount,
        uint32 channelId,
        string memory quoteToken,
        string memory receiver,
        string memory solver
    ) public {
        IERC20 t = IERC20(token);

        bytes memory hexQuoteToken = FFIHelper.ffiToHex(vm, quoteToken);
        bytes memory hexReceiver = FFIHelper.ffiToHex(vm, receiver);
        bytes memory hexSolver = FFIHelper.ffiToHex(vm, solver);

        _zkgmTokenOrderV2(zkgm, t, channelId, amount, hexQuoteToken, hexReceiver, hexSolver);
    }

    function _zkgmTokenOrderV2(
        address zkgm,
        IERC20 t,
        uint32 channelId,
        uint256 amount,
        bytes memory quoteToken,
        bytes memory receiver,
        bytes memory solver
    ) internal {
        address sender = msg.sender;

        require(t.balanceOf(sender) >= amount);

        TokenOrderV2 memory tokenOrder = TokenOrderV2({
            sender: abi.encodePacked(sender),
            receiver: receiver,
            baseToken: abi.encodePacked(t),
            baseAmount: amount,
            quoteToken: quoteToken,
            quoteAmount: amount,
            kind: ZkgmLib.TOKEN_ORDER_KIND_SOLVE,
            metadata: ZkgmLib.encodeSolverMetadata(SolverMetadata({solverAddress: solver, metadata: hex""}))
        });

        Instruction memory instruction = Instruction({
            version: ZkgmLib.INSTR_VERSION_2,
            opcode: ZkgmLib.OP_TOKEN_ORDER,
            operand: ZkgmLib.encodeTokenOrderV2(tokenOrder)
        });

        uint64 timeoutTimestamp = (uint64(block.timestamp) + 3 days) * 1_000_000_000;

        vm.startBroadcast();

        if (t.allowance(sender, zkgm) < amount) {
            t.approve(zkgm, amount);
        }
        bytes32 salt = keccak256(abi.encodePacked(sender, t, channelId, timeoutTimestamp));

        IZkgm(zkgm).send(channelId, 0, timeoutTimestamp, salt, instruction);
        vm.stopBroadcast();
    }
}
