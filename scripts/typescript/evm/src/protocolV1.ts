import { encodeAbiParameters, encodePacked, Hex, toHex } from "viem"
import { Batch, TokenOrderV1, Schema, Call, TokenOrderV1Abi } from "@unionlabs/sdk/Ucs03";
import { OP_CODE_CALL, INSTR_VERSION_ZERO } from "./protocolV2.js";
import { ethers } from "ethers";
import { ucs03abi } from "@unionlabs/sdk/evm/abi/ucs03";

export const transferInstruction = (
    baseTokenPath: bigint,
    sender: string,
    receiver: string,
    baseToken: string,
    baseAmount: bigint,
    baseTokenSymbol: string,
    baseTokenName: string,
    baseTokenDecimals: number,
    quoteToken: `0x${string}`,
    quoteAmount: bigint,
) => {

    let senderHex = sender.startsWith("0x") ? sender as Hex : toHex(sender);
    let receiverHex = receiver.startsWith("0x") ? receiver as Hex : toHex(receiver);
    let baseTokenHex = baseToken.startsWith("0x") ? baseToken as Hex : toHex(baseToken);

    let tokenOrder = TokenOrderV1.make({
        operand: [
            senderHex,
            receiverHex,
            baseTokenHex,
            baseAmount,
            baseTokenSymbol,
            baseTokenName,
            baseTokenDecimals,
            baseTokenPath,
            quoteToken, // Ensure quoteToken is a hex string
            quoteAmount,
        ]
    });


    return tokenOrder;
}


export const transferAndCallInstruction = (
    baseTokenPath: bigint,
    sender: string,
    receiver: `${string}1${string}`,
    baseToken: string,
    baseAmount: bigint,
    baseTokenSymbol: string,
    baseTokenName: string,
    baseTokenDecimals: number,
    quoteToken: `0x${string}`,
    quoteAmount: bigint,
    payload: any
) => {

    let senderHex = sender.startsWith("0x") ? sender as Hex : toHex(sender);
    let receiverHex = receiver.startsWith("0x") ? receiver as Hex : toHex(receiver);
    let baseTokenHex = baseToken.startsWith("0x") ? baseToken as Hex : toHex(baseToken);

    let fungibleAssetOrder = TokenOrderV1.make({
        operand: [
            senderHex,
            receiverHex,
            baseTokenHex,
            baseAmount,
            baseTokenSymbol,
            baseTokenName,
            baseTokenDecimals,
            baseTokenPath,
            quoteToken, // Ensure quoteToken is a hex string
            quoteAmount,
        ]
    });



    let call: Call = Call.make({
        opcode: OP_CODE_CALL,
        version: INSTR_VERSION_ZERO,
        operand: [
            receiverHex,
            false,
            senderHex,
            payload
        ]
    });


    return Batch.make({
        operand: [
            fungibleAssetOrder as Schema,
            call as Schema,
        ]
    });
}


export const encodeTokenOrderV1 = (instruction: TokenOrderV1) => {
    return encodeAbiParameters(TokenOrderV1Abi(), instruction.operand);
}


export const predictQuoteToken = async (signer: ethers.Wallet, channelId: bigint, baseToken: string) => {
    let ucs03address = "0x5fbe74a283f7954f10aa04c2edf55578811aeb03";
    const ucs03Contract = new ethers.Contract(ucs03address, ucs03abi, signer);

    let res = await ucs03Contract.predictWrappedToken(0n, channelId, baseToken);
    console.log("res", res);
}


