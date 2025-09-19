
import { TokenOrderV2Abi, TokenOrderV2, Call, CallAbi, Batch, Schema as Ucs03Schema, InstructionAbi, Instruction } from "@unionlabs/sdk/Ucs03";
import { Address, encodeAbiParameters, Hex, toHex } from "viem";
import { ChainRegistry } from "@unionlabs/sdk/ChainRegistry";
import { UniversalChainId } from "@unionlabs/sdk/schema/chain";
import { TokenOrder, Ucs03, Ucs05, Utils } from "@unionlabs/sdk";
import { Effect, pipe, Schema } from "effect";
import { getTimeoutInNanoseconds7DaysFromNow } from "@/app/lib/utils";
import { ChannelId } from "@unionlabs/sdk/schema/channel";

export const U_FROM_UNION_SOLVER_METADATA_TESTNET = "0x000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000014ba5ed44733953d79717f6269357c77718c8ba5ed0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
export const EU_FROM_UNION_SOLVER_METADATA_TESTNET = "0x000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000014e5cf13c84c0fea3236c101bd7d743d30366e5cf10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

const TOKEN_ORDER_KIND_ESCROW = 1;
const TOKEN_ORDER_KIND_SOLVE = 3;
const TOKEN_ORDER_V2_VERSION = 2;
export const OP_CODE_CALL = 1;
const OP_CODE_TOKEN_ORDER_V2 = 3;
export const INSTR_VERSION_ZERO = 0;


export const tokenOrderV2WithSolverMetadata = (
    sender: string,
    receiver: string,
    baseToken: string,
    baseAmount: bigint,
    quoteToken: `0x${string}`,
    quoteAmount: bigint,
    metadata: `0x${string}`,
) => {

    let senderHex = sender.startsWith("0x") ? sender as Hex : toHex(sender);
    let receiverHex = receiver.startsWith("0x") ? receiver as Hex : toHex(receiver);
    let baseTokenHex = baseToken.startsWith("0x") ? baseToken as Hex : toHex(baseToken);

    let tokenOrderV2: TokenOrderV2 = TokenOrderV2.make({
        opcode: OP_CODE_TOKEN_ORDER_V2,
        version: TOKEN_ORDER_V2_VERSION,
        operand: [
            senderHex,
            receiverHex,
            baseTokenHex,
            baseAmount,
            quoteToken,
            quoteAmount,
            TOKEN_ORDER_KIND_SOLVE,
            metadata]
    });

    return tokenOrderV2;
}


export const tokenOrderV2Escrow = (
    sender: string,
    receiver: string,
    baseToken: string,
    amount: bigint,
    quoteToken: `0x${string}`,
) => {

    let senderHex = sender.startsWith("0x") ? sender as Hex : toHex(sender);
    let receiverHex = receiver.startsWith("0x") ? receiver as Hex : toHex(receiver);
    let baseTokenHex = baseToken.startsWith("0x") ? baseToken as Hex : toHex(baseToken);

    let tokenOrderV2: TokenOrderV2 = TokenOrderV2.make({
        opcode: OP_CODE_TOKEN_ORDER_V2,
        version: TOKEN_ORDER_V2_VERSION,
        operand: [
            senderHex,
            receiverHex,
            baseTokenHex,
            amount,
            quoteToken,
            amount,
            TOKEN_ORDER_KIND_ESCROW,
            toHex("")]
    });

    return tokenOrderV2;
}


export const encodeTokenOrderV2 = (instruction: TokenOrderV2) => {
    return encodeAbiParameters(TokenOrderV2Abi(), instruction.operand);
}


export const encodeInstruction = (instruction: Instruction) => {
    return encodeAbiParameters(
        InstructionAbi(),
        [instruction.version, instruction.opcode, instruction.operand] as const);
}


export const callInstruction = (
    sender: string,
    contractAddress: string,
    payload: `0x${string}`,
) => {

    let senderHex = sender.startsWith("0x") ? sender as Hex : toHex(sender);
    let contractAddressHex = contractAddress.startsWith("0x") ? contractAddress as Hex : toHex(contractAddress);

    let call: Call = Call.make({
        opcode: OP_CODE_CALL,
        version: INSTR_VERSION_ZERO,
        operand: [
            senderHex,
            false,
            contractAddressHex,
            payload
        ]
    });

    return call;
}


export const encodeCall = (call: Call) => {
    return encodeAbiParameters(CallAbi(), call.operand);
}




export const getInstructionBatch = (
    instructions: [Ucs03Schema, ...Ucs03Schema[]]
) => {

    const batch = Batch.make({
        operand: instructions
    })

    return batch;
}

interface GetSendbackCallMsgParams {
    sender: Address
    receiver: string
    minAmount: bigint
    baseToken: string
    quoteToken: string
    metadata: `0x${string}`
    channel_id: ChannelId
    ucs03: `${string}1${string}`
}

const JsonFromBase64 = Schema.compose(
    Schema.StringFromBase64,
    Schema.parseJson(),
)

export const getSendbackCallMsg = (params: GetSendbackCallMsgParams) =>
    Effect.gen(function* () {

        const UCS03_ZKGM = Ucs05.CosmosDisplay.make({
            address: params.ucs03,
        });
        const SENDER = Ucs05.EvmDisplay.make({
            address: params.sender,
        })
        const MIN_MINT_AMOUNT = params.minAmount;
        const ETHEREUM_CHAIN_ID = UniversalChainId.make("ethereum.17000");
        const UNION_CHAIN_ID = UniversalChainId.make("union.union-testnet-10");

        const ethereumChain = yield* ChainRegistry.byUniversalId(ETHEREUM_CHAIN_ID);
        const unionChain = yield* ChainRegistry.byUniversalId(UNION_CHAIN_ID);

        const salt = yield* Utils.generateSalt("cosmos");

        const sendCall = yield* pipe(
            TokenOrder.make({
                source: unionChain,
                destination: ethereumChain,
                sender: Ucs05.CosmosDisplay.make({
                    address: params.receiver as '`${string}1${string}`',
                }),
                receiver: SENDER,
                baseToken: params.baseToken,
                baseAmount: MIN_MINT_AMOUNT,
                quoteToken: params.quoteToken,
                quoteAmount: MIN_MINT_AMOUNT,
                kind: "solve",
                metadata: params.metadata,
                version: 2,
            }),
            Effect.flatMap(TokenOrder.encodeV2),
            Effect.flatMap(Schema.encode(Ucs03.Ucs03WithInstructionFromHex)),
            Effect.map((instruction) => ({
                send: {
                    channel_id: params.channel_id,
                    timeout_height: BigInt(0).toString(),
                    timeout_timestamp: getTimeoutInNanoseconds7DaysFromNow().toString(),
                    salt,
                    instruction,
                },
            } as const)),
            Effect.flatMap(Schema.encode(JsonFromBase64)),
            Effect.map((msg) => ({
                wasm: {
                    execute: {
                        contract_addr: UCS03_ZKGM.address,
                        msg,
                        funds: [],
                    },
                },
            })),
        )
        return sendCall
    }).pipe(
        Effect.provide(ChainRegistry.Default),
    );



