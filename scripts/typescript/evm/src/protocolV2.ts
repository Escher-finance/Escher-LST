import {
  TokenOrderV2Abi,
  TokenOrderV2,
  Call,
  CallAbi,
  Batch,
  Schema as Ucs03Schema,
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
import { Effect, pipe, Schema } from "effect";
import { getTimeoutInNanoseconds7DaysFromNow } from "./utils.js";
import { ChannelId } from "@unionlabs/sdk/schema/channel";
import { ethers } from "ethers";

const U_TO_UNION_SOLVER_METADATA_TESTNET =
  "0x000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000040756e696f6e3175757575757575757539756e3271706b73616d37726c747470786338646337366d63706868736d70333970786a6e7376727463717679763537720000000000000000000000000000000000000000000000000000000000000000";

const TOKEN_ORDER_KIND_UNESCROW = 2;
const TOKEN_ORDER_KIND_SOLVE = 3;
const TOKEN_ORDER_V2_VERSION = 2;
export const OP_CODE_CALL = 1;
const OP_CODE_TOKEN_ORDER_V2 = 3;
export const INSTR_VERSION_ZERO = 0;

export const tokenOrderV2Unescrow = (
  sender: string,
  receiver: string,
  baseToken: string,
  baseAmount: bigint,
  quoteToken: `0x${string}`,
  quoteAmount: bigint
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

export const tokenOrderV2 = (
  sender: string,
  receiver: string,
  baseToken: string,
  baseAmount: bigint,
  quoteToken: `0x${string}`,
  quoteAmount: bigint
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
      U_TO_UNION_SOLVER_METADATA_TESTNET,
    ],
  });

  return tokenOrderV2;
};

export const encodeTokenOrderV2 = (instruction: TokenOrderV2) => {
  return encodeAbiParameters(TokenOrderV2Abi(), instruction.operand);
};

export const callInstruction = (
  sender: string,
  contractAddress: string,
  payload: `0x${string}`
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
  instructions: [Ucs03Schema, ...Ucs03Schema[]]
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
  Schema.parseJson()
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

    const ethereumChain = yield* ChainRegistry.byUniversalId(ETHEREUM_CHAIN_ID);
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
          } as const)
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
      }))
    );
    return sendCall;
  }).pipe(Effect.provide(ChainRegistry.Default));

export const getBabylonSendbackCallMsg = (params: GetSendbackCallMsgParams) =>
  Effect.gen(function* () {
    const UCS03_ZKGM = Ucs05.CosmosDisplay.make({
      address: params.ucs03,
    });
    const SENDER = Ucs05.EvmDisplay.make({
      address: params.sender,
    });
    const MIN_MINT_AMOUNT = params.minAmount;
    const ETHEREUM_CHAIN_ID = UniversalChainId.make("ethereum.11155111");
    const BABYLON_CHAIN_ID = UniversalChainId.make("babylon.bbn-test-5");

    const ethereumChain = yield* ChainRegistry.byUniversalId(ETHEREUM_CHAIN_ID);
    const babylonChain = yield* ChainRegistry.byUniversalId(BABYLON_CHAIN_ID);

    const salt = yield* Utils.generateSalt("cosmos");

    const sendCall = yield* pipe(
      TokenOrder.make({
        source: babylonChain,
        destination: ethereumChain,
        sender: Ucs05.CosmosDisplay.make({
          address: params.receiver as "`${string}1${string}`",
        }),
        receiver: SENDER,
        baseToken: params.baseToken,
        baseAmount: MIN_MINT_AMOUNT,
        quoteToken: params.quoteToken,
        quoteAmount: MIN_MINT_AMOUNT,
        kind: "escrow",
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
          } as const)
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
      }))
    );
    return sendCall;
  }).pipe(Effect.provide(ChainRegistry.Default));

export interface ZkgmERC20InitializeParams {
  authority: `0x${string}`;
  zkgm: `0x${string}`;
  name: string;
  symbol: string;
  decimals: number;
}

export function encodeZkgmERC20Initialize(
  params: ZkgmERC20InitializeParams
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

export interface TokenMetadata {
  implementation: `0x${string}`; // bytes type in Solidity
  initializer: `0x${string}`; // bytes type in Solidity
}

const predictAbi = [
  {
    type: "function",
    name: "predictWrappedTokenV2",
    inputs: [
      { name: "path", type: "uint256", internalType: "uint256" },
      { name: "channel", type: "uint32", internalType: "uint32" },
      { name: "token", type: "bytes", internalType: "bytes" },
      {
        name: "metadata",
        type: "tuple",
        internalType: "struct TokenMetadata",
        components: [
          { name: "implementation", type: "bytes", internalType: "bytes" },
          {
            name: "initializer",
            type: "bytes",
            internalType: "bytes",
          },
        ],
      },
    ],
    outputs: [
      { name: "", type: "address", internalType: "address" },
      {
        name: "",
        type: "bytes32",
        internalType: "bytes32",
      },
    ],
    stateMutability: "view",
  },
];

const zkgmERC20Abi = parseAbi([
  "function initialize(address authority, address minter, string name, string symbol, uint8 decimals)",
]);

export const predictWrappedTokenV2 = async (
  signer: ethers.Wallet,
  channelId: bigint,
  baseToken: string
) => {
  let ucs03address = "0x5fbe74a283f7954f10aa04c2edf55578811aeb03";

  const initializerCallData = encodeZkgmERC20Initialize({
    authority: "0x40cdff51ae7487e0b4a4d6e5f86eb15fb7c1d9f4",
    zkgm: "0x5fbe74a283f7954f10aa04c2edf55578811aeb03",
    name: "eBABY",
    symbol: "ebbn",
    decimals: 6,
  });

  const implementation = encodePacked(
    ["address"],
    ["0xAf739F34ddF951cBC24fdbBa4f76213688E13627"]
  );

  let metadata = [implementation, initializerCallData];
  const ucs03Contract = new ethers.Contract(ucs03address, predictAbi, signer);
  let res = await ucs03Contract.predictWrappedTokenV2(
    0n,
    channelId,
    baseToken,
    metadata
  );
  console.log("res", res);
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
  params: GetAddressFromEvmParams
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
      ] as const)
    ),
    Effect.map((encoded) => keccak256(encoded, "bytes"))
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
    Effect.tryPromise(() => globalThis.crypto.subtle.digest("SHA-256", data))
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
      Schema.decode(Ucs05.Bech32FromCanonicalBytesWithPrefix("bbn"))
    )
  );

  return Ucs05.CosmosDisplay.make({ address });
});
