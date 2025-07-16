import { Hex, toHex } from "viem";
import { Instruction } from "@unionlabs/sdk/ucs03";


declare global {
    interface BigInt {
        toJSON(): Number;
    }
}

BigInt.prototype.toJSON = function () { return Number(this) }

export interface TransferAndCallIntent {
    sender: string;
    receiver: string;
    baseAmount: bigint;
    baseToken: string
    baseTokenSymbol: string;
    baseTokenName: string;
    quoteToken: `0x${string}`;
    quoteAmount: bigint,
    baseTokenPath: bigint;
    payload: any;
}

export interface TransferIntent {
    sender: string;
    receiver: string;
    baseAmount: bigint;
    baseToken: string
    baseTokenSymbol: string;
    baseTokenName: string;
    quoteToken: `0x${string}`;
    quoteAmount: bigint,
    baseTokenPath: bigint;
    baseTokenDecimals: number;
}


export const transferAndCallInstruction = ({
    sender,
    receiver,
    baseToken,
    baseAmount,
    baseTokenSymbol,
    baseTokenName,
    quoteToken,
    quoteAmount,
    baseTokenPath,
    payload
}: TransferAndCallIntent
) => {
    let baseTokenDecimals = 6;
    let senderHex = sender.startsWith("0x") ? sender as Hex : toHex(sender);
    let receiverHex = receiver.startsWith("0x") ? receiver as Hex : toHex(receiver);
    let baseTokenHex = baseToken.startsWith("0x") ? baseToken as Hex : toHex(baseToken);

    let fungibleAssetOrder = Instruction.FungibleAssetOrder.make({
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


    return new Instruction.Batch({
        operand: [
            fungibleAssetOrder,
            new Instruction.Multiplex({
                operand: [
                    senderHex as `0x${string}`, // Ensure sender is a hex string
                    false,
                    toHex(receiver), // Ensure target contract is a hex string,
                    toHex(JSON.stringify(payload)),
                ]
            })
        ]
    });
}



export const transferInstruction = ({
    baseTokenPath,
    sender,
    receiver,
    baseToken,
    baseAmount,
    baseTokenSymbol,
    baseTokenName,
    quoteToken,
    quoteAmount,
    baseTokenDecimals
}: TransferIntent
) => {
    let senderHex = sender.startsWith("0x") ? sender as Hex : toHex(sender);
    let receiverHex = receiver.startsWith("0x") ? receiver as Hex : toHex(receiver);
    let baseTokenHex = baseToken.startsWith("0x") ? baseToken as Hex : toHex(baseToken);

    let fungibleAssetOrder = Instruction.FungibleAssetOrder.make({
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


    return new Instruction.Batch({
        operand: [
            fungibleAssetOrder,
        ]
    });
}


export function getSalt() {
    const rawSalt = new Uint8Array(32)
    crypto.getRandomValues(rawSalt)
    const salt = toHex(rawSalt);
    return salt
}

export function getTimeoutInNanoseconds24HoursFromNow(): bigint {
    const millisecondsNow = Date.now() // current time in ms
    const millisecondsIn24Hours = 24 * 60 * 60 * 1000 * 3 // 24 hours in ms * 3
    const totalMilliseconds = millisecondsNow + millisecondsIn24Hours
    return BigInt(totalMilliseconds) * BigInt(1_000_000) // convert ms to ns
}