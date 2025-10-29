import {
    TokenOrderV2Abi,
    TokenOrderV2,
    Call,
    CallAbi,
    Batch,
    BatchAbi,
    Schema as Ucs03Schema,
    InstructionAbi,
    Instruction,
    TokenMetadataAbi,
} from "@unionlabs/sdk/Ucs03";
import {
    Address,
    bytesToHex,
    encodeAbiParameters,
    encodeFunctionData,
    encodePacked,
    Hex,
    parseAbi,
    fromHex,
    toHex,
    keccak256,
} from "viem";
import { ChainRegistry } from "@unionlabs/sdk/ChainRegistry";
import { UniversalChainId } from "@unionlabs/sdk/schema/chain";
import { TokenOrder, Ucs03, Ucs05, Utils } from "@unionlabs/sdk";
import { Effect, Schema, pipe } from "effect";
import { getSalt, getTimeoutInNanoseconds7DaysFromNow } from "@/app/lib/utils";
import { ChannelId } from "@unionlabs/sdk/schema/channel";
import { HexFromJson } from "@unionlabs/sdk/schema/hex";
import Networks, {
    ChainConfig,
    SupportedNetworks,
} from "@/config/networks.config";

export const U_FROM_UNION_SOLVER_METADATA_TESTNET =
    "0x000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000014ba5ed44733953d79717f6269357c77718c8ba5ed0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
export const EU_FROM_UNION_SOLVER_METADATA_TESTNET =
    "0x000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000014e5cf13c84c0fea3236c101bd7d743d30366e5cf10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

const TOKEN_ORDER_KIND_INITIALIZE = 0;
const TOKEN_ORDER_KIND_ESCROW = 1;
const TOKEN_ORDER_KIND_UNESCROW = 2;
const TOKEN_ORDER_KIND_SOLVE = 3;
const TOKEN_ORDER_V2_VERSION = 2;
export const OP_CODE_CALL = 1;
const OP_CODE_TOKEN_ORDER_V2 = 3;
export const INSTR_VERSION_ZERO = 0;

export const BYTECODE_BASE_CHECKSUM =
    "0xec827349ed4c1fec5a9c3462ff7c979d4c40e7aa43b16ed34469d04ff835f2a1" as const;

export const MODULE_HASH =
    "0x120970d812836f19888625587a4606a5ad23cef31c8684e601771552548fc6b9" as const;

export interface TokenMetadata {
    implementation: `0x${string}`; // bytes type in Solidity
    initializer: `0x${string}`; // bytes type in Solidity
}

export const tokenOrderV2WithSolverMetadata = (
    sender: string,
    receiver: string,
    baseToken: string,
    baseAmount: bigint,
    quoteToken: `0x${string}`,
    quoteAmount: bigint,
    metadata: `0x${string}`,
) => {
    let senderHex = sender.startsWith("0x") ? (sender as Hex) : toHex(sender);
    let receiverHex = receiver.startsWith("0x")
        ? (receiver as Hex)
        : toHex(receiver);
    let baseTokenHex = baseToken.startsWith("0x")
        ? (baseToken as Hex)
        : toHex(baseToken);

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
            metadata,
        ],
    });

    return tokenOrderV2;
};

function encodeTokenMetadataWithAbi(metadata: TokenMetadata): `0x${string}` {
    return encodeAbiParameters(TokenMetadataAbi(), [
        metadata.implementation,
        metadata.initializer,
    ]);
}

export const tokenOrderV2Escrow = (
    sender: string,
    receiver: string,
    baseToken: string,
    amount: bigint,
    quoteToken: `0x${string}`,
) => {
    let senderHex = sender.startsWith("0x") ? (sender as Hex) : toHex(sender);
    let receiverHex = receiver.startsWith("0x")
        ? (receiver as Hex)
        : toHex(receiver);
    let baseTokenHex = baseToken.startsWith("0x")
        ? (baseToken as Hex)
        : toHex(baseToken);

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
            toHex(""),
        ],
    });

    return tokenOrderV2;
};

export const tokenOrderV2Unescrow = (
    sender: string,
    receiver: string,
    baseToken: string,
    baseAmount: bigint,
    quoteToken: `0x${string}`,
    quoteAmount: bigint,
) => {
    let senderHex = sender.startsWith("0x") ? (sender as Hex) : toHex(sender);
    let receiverHex = receiver.startsWith("0x")
        ? (receiver as Hex)
        : toHex(receiver);
    let baseTokenHex = baseToken.startsWith("0x")
        ? (baseToken as Hex)
        : toHex(baseToken);

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
            TOKEN_ORDER_KIND_UNESCROW,
            toHex(""),
        ],
    });

    return tokenOrderV2;
};

export const tokenOrderV2Initialize = (
    sender: string,
    receiver: string,
    baseToken: string,
    amount: bigint,
    quoteToken: `0x${string}`,
) => {
    let senderHex = sender.startsWith("0x") ? (sender as Hex) : toHex(sender);
    let receiverHex = receiver.startsWith("0x")
        ? (receiver as Hex)
        : toHex(receiver);
    let baseTokenHex = baseToken.startsWith("0x")
        ? (baseToken as Hex)
        : toHex(baseToken);

    const initializerCallData = encodeZkgmERC20Initialize({
        authority: "0x40cdff51ae7487e0b4a4d6e5f86eb15fb7c1d9f4",
        zkgm: "0x5fbe74a283f7954f10aa04c2edf55578811aeb03",
        name: "eBABY",
        symbol: "ebbn",
        decimals: 6,
    });

    const implementation = encodePacked(
        ["address"],
        ["0xAf739F34ddF951cBC24fdbBa4f76213688E13627"],
    );

    let tokenMetadata: TokenMetadata = {
        implementation,
        initializer: initializerCallData,
    };

    let tokenMetadataBytes = encodeTokenMetadataWithAbi(tokenMetadata);

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
            TOKEN_ORDER_KIND_INITIALIZE,
            tokenMetadataBytes,
        ],
    });

    return tokenOrderV2;
};

export const zkgmERC20Abi = parseAbi([
    "function initialize(address authority, address minter, string name, string symbol, uint8 decimals)",
]);

export interface ZkgmERC20InitializeParams {
    authority: `0x${string}`;
    zkgm: `0x${string}`;
    name: string;
    symbol: string;
    decimals: number;
}

export function encodeZkgmERC20Initialize(
    params: ZkgmERC20InitializeParams,
): `0x${string}` {
    return encodeFunctionData({
        abi: zkgmERC20Abi,
        functionName: "initialize",
        args: [
            params.authority,
            params.zkgm,
            params.name,
            params.symbol,
            params.decimals,
        ],
    });
}

export const encodeTokenOrderV2 = (instruction: TokenOrderV2) => {
    return encodeAbiParameters(TokenOrderV2Abi(), instruction.operand);
};

export const encodeInstruction = (instruction: Instruction) => {
    return encodeAbiParameters(InstructionAbi(), [
        instruction.version,
        instruction.opcode,
        instruction.operand,
    ] as const);
};

export const callInstruction = (
    sender: string,
    contractAddress: string,
    payload: `0x${string}`,
) => {
    let senderHex = sender.startsWith("0x") ? (sender as Hex) : toHex(sender);
    let contractAddressHex = contractAddress.startsWith("0x")
        ? (contractAddress as Hex)
        : toHex(contractAddress);

    let call: Call = Call.make({
        opcode: OP_CODE_CALL,
        version: INSTR_VERSION_ZERO,
        operand: [senderHex, false, contractAddressHex, payload],
    });

    return call;
};

export const encodeCall = (call: Call) => {
    return encodeAbiParameters(CallAbi(), call.operand);
};

export const getInstructionBatch = (
    instructions: [Ucs03Schema, ...Ucs03Schema[]],
) => {
    const batch = Batch.make({
        operand: instructions,
    });

    return batch;
};

interface GetSendbackCallMsgParams {
    sender: Address;
    receiver: string;
    minAmount: bigint;
    baseToken: string;
    quoteToken: string;
    metadata: `0x${string}`;
    channel_id: ChannelId;
    ucs03: `${string}1${string}`;
}

const JsonFromBase64 = Schema.compose(
    Schema.StringFromBase64,
    Schema.parseJson(),
);

export const getSendbackCallMsg = (params: GetSendbackCallMsgParams) =>
    Effect.gen(function* () {
        const UCS03_ZKGM = Ucs05.CosmosDisplay.make({
            address: params.ucs03,
        });
        const SENDER = Ucs05.EvmDisplay.make({
            address: params.sender,
        });
        const MIN_MINT_AMOUNT = params.minAmount;
        const ETHEREUM_CHAIN_ID = UniversalChainId.make("ethereum.17000");
        const UNION_CHAIN_ID = UniversalChainId.make("union.union-testnet-10");

        const ethereumChain =
            yield* ChainRegistry.byUniversalId(ETHEREUM_CHAIN_ID);
        const unionChain = yield* ChainRegistry.byUniversalId(UNION_CHAIN_ID);

        const salt = yield* Utils.generateSalt("cosmos");

        const sendCall = yield* pipe(
            TokenOrder.make({
                source: unionChain,
                destination: ethereumChain,
                sender: Ucs05.CosmosDisplay.make({
                    address: params.receiver as "`${string}1${string}`",
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
            Effect.map(
                (instruction) =>
                    ({
                        send: {
                            channel_id: params.channel_id,
                            timeout_height: BigInt(0).toString(),
                            timeout_timestamp:
                                getTimeoutInNanoseconds7DaysFromNow().toString(),
                            salt,
                            instruction,
                        },
                    }) as const,
            ),
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
        );
        return sendCall;
    }).pipe(Effect.provide(ChainRegistry.Default));

export const createUnbondPayload = ({
    lst_address,
    recipient_address,
    amount,
    channel_id,
}: {
    lst_address: string;
    recipient_address: string;
    amount: string;
    channel_id: number;
}) => {
    const unbondMsg = {
        remote_unbond: {
            amount,
            recipient: {
                ibc: {
                    address: recipient_address,
                    channel_id,
                },
            },
        },
    } as const;

    const payload = {
        wasm: {
            execute: {
                contract_addr: lst_address,
                msg: Buffer.from(JSON.stringify(unbondMsg)).toString("base64"),
                funds: [],
            },
        },
    };

    return payload;
};

export const createIBCUnbondPayload = ({
    lst_address,
    recipient_address,
    amount,
    ibc_channel_id,
}: {
    lst_address: string;
    recipient_address: string;
    amount: string;
    ibc_channel_id: string;
}) => {
    const unbondMsg = {
        unbond: {
            amount,
            recipient: {
                ibc: {
                    address: recipient_address,
                    ibc_channel_id,
                },
            },
        },
    } as const;

    console.log("unbondMsg", JSON.stringify(unbondMsg));
    console.log("lst_address", lst_address);

    const payload = {
        wasm: {
            execute: {
                contract_addr: lst_address,
                msg: Buffer.from(JSON.stringify(unbondMsg)).toString("base64"),
                funds: [],
            },
        },
    };

    return payload;
};

export const createIncreaseAllowance = (
    spender: string,
    amount: string,
    contractAddress: string,
) => {
    // Allowance Call
    const allowanceMsg = {
        increase_allowance: {
            spender,
            amount,
        },
    } as const;
    const allowancePayload = {
        wasm: {
            execute: {
                contract_addr: contractAddress,
                msg: Buffer.from(JSON.stringify(allowanceMsg)).toString(
                    "base64",
                ),
                funds: [],
            },
        },
    };

    return allowancePayload;
};

export const getIBCUnbondCallsInstruction = async (
    sender: string,
    amount: string,
    ibc_channel_id: string,
    proxy_address: string,
    lst_address: string,
    cw20_address: string,
) => {
    // give allowance to lst contract to transfer ebaby from proxy contract
    let allowancePayload = createIncreaseAllowance(
        lst_address,
        amount,
        cw20_address,
    );

    console.log(JSON.stringify(allowancePayload));

    // call unbond to lst contract
    let unbondPayload = createIBCUnbondPayload({
        lst_address,
        recipient_address: sender,
        amount,
        ibc_channel_id,
    });

    // Calls
    const callsPayload = [allowancePayload, unbondPayload];

    const calls = callInstruction(
        sender.toLowerCase(),
        toHex(proxy_address),
        Schema.decodeSync(HexFromJson)(callsPayload),
    );

    return calls;
};

export const unbondSendToIBC = async (
    sender: string,
    amount: bigint,
    proxyAddress: string,
    targetChain: "babylon",
    network: ChainConfig,
) => {
    let salt = getSalt();
    console.log(salt);

    if (
        !network?.escher?.stakedBaseToken ||
        !network?.escher?.channel[targetChain].stakedQuoteToken
    ) {
        throw Error("no staked base token or staked quote token");
    }
    console.log("amount:", amount);

    let tokenOrder = tokenOrderV2Unescrow(
        sender.toLowerCase(),
        proxyAddress,
        network?.escher?.stakedBaseToken,
        amount,
        network?.escher?.channel[targetChain].stakedQuoteToken as `0x${string}`,
        amount,
    );

    let targetChainName: SupportedNetworks =
        network?.chainName.indexOf("mainnet") != -1
            ? `${targetChain}-mainnet`
            : `${targetChain}-testnet`;

    console.log(targetChainName, targetChainName);
    let calls = await getIBCUnbondCallsInstruction(
        sender,
        amount.toString(),
        network?.escher?.channel[targetChain].destinationIbcChannelId,
        proxyAddress,
        Networks[targetChainName].contracts.lst,
        Networks[targetChainName].contracts.cw20,
    );

    // // Batch Call
    const batchCall: Batch = getInstructionBatch([tokenOrder]);

    const batchInstructions: [
        { version: number; opcode: number; operand: `0x${string}` }[],
    ] = [
        [
            // Tokenorder, send eBaby token
            {
                version: tokenOrder.version,
                opcode: tokenOrder.opcode,
                operand: encodeTokenOrderV2(tokenOrder),
            },

            // Bond message
            {
                version: calls.version,
                opcode: calls.opcode,
                operand: encodeCall(calls),
            },
        ],
    ];
    const batchOperand = encodeAbiParameters(BatchAbi(), batchInstructions);

    console.log({ batchInstructions });

    return Instruction.make({
        version: batchCall.version,
        opcode: batchCall.opcode,
        operand: batchOperand,
    });
};

export interface GetAddressFromEvmParams {
    path: bigint;
    channel: ChannelId;
    sender: `0x${string}`;
    ucs03: `${string}1${string}`;
    bytecode_base_checksum: `0x${string}`;
    module_hash: `0x${string}`;
}

export const getAddressFromEvm = Effect.fn(function* (
    params: GetAddressFromEvmParams,
) {
    const UCS03_ZKGM = Ucs05.CosmosDisplay.make({
        address: params.ucs03,
    });
    const canonical_zkgm = Ucs05.anyDisplayToCanonical(UCS03_ZKGM);

    const abi = [
        {
            name: "path",
            type: "uint256",
            internalType: "uint256",
        },
        {
            name: "channelId",
            type: "uint32",
            internalType: "uint32",
        },
        {
            name: "sender",
            type: "bytes",
            internalType: "bytes",
        },
    ] as const;

    const salt = yield* pipe(
        Effect.try(() =>
            encodeAbiParameters(abi, [
                params.path,
                params.channel,
                params.sender,
            ] as const),
        ),
        Effect.map((encoded) => keccak256(encoded, "bytes")),
    );

    /**
     * `n` from U64 to big-endian bytes
     */
    const u64toBeBytes = (n: bigint) => {
        const buffer = new ArrayBuffer(8);
        const view = new DataView(buffer);
        view.setBigUint64(0, n);
        return new Uint8Array(view.buffer);
    };

    const sha256 = Effect.fn((data: any) =>
        Effect.tryPromise(() =>
            globalThis.crypto.subtle.digest("SHA-256", data),
        ),
    );

    const address = yield* pipe(
        Uint8Array.from([
            ...fromHex(params.module_hash, "bytes"),
            ...new TextEncoder().encode("wasm"),
            0, // null byte
            ...u64toBeBytes(BigInt(32)), // checksum len as 64-bit big endian bytes of int
            ...fromHex(params.bytecode_base_checksum, "bytes"),
            ...u64toBeBytes(BigInt(32)), // creator canonical addr len
            ...fromHex(canonical_zkgm, "bytes"),
            ...u64toBeBytes(BigInt(32)), // len
            ...salt,
            ...u64toBeBytes(BigInt(0)),
        ]),
        sha256,
        Effect.map((r) => new Uint8Array(r)),
        Effect.map(bytesToHex),
        Effect.flatMap(
            Schema.decode(Ucs05.Bech32FromCanonicalBytesWithPrefix("bbn")),
        ),
    );

    return Ucs05.CosmosDisplay.make({ address });
});
