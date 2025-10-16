import { getSalt, getTimeoutInNanoseconds7DaysFromNow } from "./utils.js";
import {
  tokenOrderV2,
  getSendbackCallMsg,
  getBabylonSendbackCallMsg,
  callInstruction,
  getInstructionBatch,
  encodeTokenOrderV2,
  encodeCall,
  tokenOrderV2Unescrow,
} from "./protocolV2.js";
import { Address, encodeAbiParameters, toHex } from "viem";
import { ethers } from "ethers";
import { erc20Abi } from "viem";
import { ucs03abi } from "@unionlabs/sdk/evm/abi/ucs03";
import { Effect, Schema } from "effect";
import { HexFromJson } from "@unionlabs/sdk/schema/hex";
import { Brand } from "effect/Brand";
import { CosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { Batch, BatchAbi } from "@unionlabs/sdk/Ucs03";

const UNION_RPC_URL = "https://rpc.rpc-node.union-testnet-10.union.build";
const UNION_UCS03 =
  "union1336jj8ertl8h7rdvnz4dh5rqahd09cy0x43guhsxx6xyrztx292qpe64fh";
const UNION_LST_CONTRACT =
  "union1d2r4ecsuap4pujrlf3nz09vz8eha8y0z25knq0lfxz4yzn83v6kq0jxsmk";
const EU_ON_UNION_CONTRACT_ADDRESS =
  "union1eueueueu9var4yhdruyzkjcsh74xzeug6ckyy60hs0vcqnzql2hq0lxc2f";
const EU_ERC20 = "0xe5cf13c84c0fea3236c101bd7d743d30366e5cf1";
const UNION_ZKGM_MINTER_ADDRESS =
  "union1t5awl707x54k6yyx7qfkuqp890dss2pqgwxh07cu44x5lrlvt4rs8hqmk0";
const EU_FROM_UNION_SOLVER_METADATA =
  "0x000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000014e5cf13c84c0fea3236c101bd7d743d30366e5cf10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

const HOLESKY_TO_UNION_CHANNEL_ID = 6;
const UNION_TO_HOLESKY_CHANNEL_ID = 20;

const BABYLON_TO_SEPOLIA_CHANNEL_ID = 1;
const BABYLON_TO_HOLESKY_CHANNEL_ID = 2;
const BABYLON_TO_ETHEREUM_CHANNEL_ID = 3;

const BABYLON_SOURCE_CHANNEL_ID: Record<string, number> = {
  sepolia: BABYLON_TO_SEPOLIA_CHANNEL_ID,
  holesky: BABYLON_TO_HOLESKY_CHANNEL_ID,
  ethereum: BABYLON_TO_ETHEREUM_CHANNEL_ID,
};

let ETH_UCS03 = "0x5fbe74a283f7954f10aa04c2edf55578811aeb03";
const U_ERC20 = "0xba5ed44733953d79717f6269357c77718c8ba5ed"; //erc20 U on Eth

const SEPOLIA_BBN_ERC20 = "0xbd030914ab8d7ab1bd626f09e47c7cc2881550a3";
const HOLESKY_BBN_ERC20 = "0x77b99a27a5fed3bc8fb3e2f1063181f82ec48637";
const ETHEREUM_BBN_ERC20 = "0xe53dcec07d16d88e386ae0710e86d9a400f83c31";

const BABY_ERC20: Record<string, string> = {
  sepolia: SEPOLIA_BBN_ERC20,
  holesky: HOLESKY_BBN_ERC20,
  ethereum: ETHEREUM_BBN_ERC20,
};

let AU_ON_UNION_QUOTE_TOKEN: Address = "0x6175"; //Denom name in au in hex

let BBN_ON_BABYLON_QUOTE_TOKEN: Address = "0x7562626e"; // Denom ubbn in hex
let EBABY_ON_BABYLON_QUOTE_TOKEN: Address =
  "0x62626e31636e7833347038327a6e677130757561656e64736e65307834733567736d376770776b326573387a6b38727a38746e6a39333871717971386639"; // Denom ebaby in hex

const BABYLON_LST_CONTRACT =
  "bbn1ug4tume0pw6d4u7r6rhae6cp3udyrv7cr0angx8qegw7ur25sdxq4krcss";

const BABYLON_MAINNET_LST_CONTRACT =
  "bbn1m7zr5jw4k9z22r9ajggf4ucalwy7uxvu9gkw6tnsmv42lvjpkwasagek5g"; //counter for testing only

const BABYLON_RPC_URL = "https://babylon-testnet-rpc.polkachu.com";

const BABYLON_MAINNET_RPC_URL = "https://babylon-rpc.polkachu.com/";

const EBABY_ON_BABYLON_TESTNET =
  "bbn1cnx34p82zngq0uuaendsne0x4s5gsm7gpwk2es8zk8rz8tnj938qqyq8f9";
const EBABY_ON_BABYLON_MAINNET =
  "bbn1s7jzz7cyuqmy5xpr07yepka5ngktexsferu2cr4xeww897ftj77sv30f5s";

const EBABY_ERC20_SEPOLIA = "0x4f8514fb579baf4c7c0e5486ab6793333552c534";
const EBABY_ERC20_HOLESKY = "0xe5551306179361cfd169435c4f27445e81ba630a";
const EBABY_ERC20_ETHEREUM = "0x70df20655b3e294facb436383435754dbee3cd70";

const EBABY_ERC20 = {
  sepolia: EBABY_ERC20_SEPOLIA,
  holesky: EBABY_ERC20_HOLESKY,
  ethereum: EBABY_ERC20_ETHEREUM,
};

const BABYLON_UCS03 =
  "bbn1336jj8ertl8h7rdvnz4dh5rqahd09cy0x43guhsxx6xyrztx292q77945h";
const BABYLON_ZKGM_MINTER_ADDRESS =
  "bbn1sakazthycqgzer50nqgr5ta4vy3gwz8wxla3s8rd8pql4ctmz5qssg39sf";

declare global {
  interface BigInt {
    toJSON(): string;
  }
}

BigInt.prototype.toJSON = function () {
  return this.toString();
};

export const createBondPayload = ({
  lst_address,
  mint_to_address,
  amount,
  min_mint_amount,
  denom,
}: {
  lst_address: string;
  mint_to_address: string;
  amount: string;
  min_mint_amount: string;
  denom: string;
}) => {
  const bondMsg = {
    bond: {
      mint_to_address,
      min_mint_amount,
    },
  } as const;

  const bondPayload = {
    wasm: {
      execute: {
        contract_addr: lst_address,
        msg: Buffer.from(JSON.stringify(bondMsg)).toString("base64"),
        funds: [{ denom, amount }],
      },
    },
  };

  return bondPayload;
};

export const createBabylonBondPayload = ({
  lst_address,
  mint_to_address,
  amount,
  min_mint_amount,
  denom,
}: {
  lst_address: string;
  mint_to_address: string;
  amount: string;
  min_mint_amount: string;
  denom: string;
}) => {
  const bondMsg = {
    remote_bond: {
      min_mint_amount,
      mint_to_address,
    },
  } as const;

  const bondPayload = {
    wasm: {
      execute: {
        contract_addr: lst_address,
        msg: Buffer.from(JSON.stringify(bondMsg)).toString("base64"),
        funds: [{ denom, amount }],
      },
    },
  };

  return bondPayload;
};

export const createBabylonUnbondPayload = ({
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
        zkgm: {
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

export const createIncreaseAllowance = (
  spender: string,
  amount: string,
  contractAddress: string
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
        msg: Buffer.from(JSON.stringify(allowanceMsg)).toString("base64"),
        funds: [],
      },
    },
  };

  return allowancePayload;
};

export const getUnionCallsInstruction = async (
  sender: string,
  amount: string,
  min_mint_amount: bigint,
  proxy_address: string,
  channel_id: number
) => {
  let bondPayload = createBondPayload({
    lst_address: UNION_LST_CONTRACT,
    mint_to_address: proxy_address,
    amount,
    min_mint_amount: min_mint_amount.toString(),
    denom: "au",
  });

  console.log(JSON.stringify(bondPayload));

  let allowancePayload = createIncreaseAllowance(
    UNION_ZKGM_MINTER_ADDRESS,
    min_mint_amount.toString(),
    EU_ON_UNION_CONTRACT_ADDRESS
  );

  console.log(JSON.stringify(allowancePayload));

  const sendBackCallEffectPayload = await Effect.runPromise(
    getSendbackCallMsg({
      baseToken: EU_ON_UNION_CONTRACT_ADDRESS,
      channel_id: channel_id as number & Brand<"ChannelId">,
      metadata: EU_FROM_UNION_SOLVER_METADATA,
      minAmount: min_mint_amount,
      quoteToken: EU_ERC20 as `0x${string}`,
      receiver: proxy_address,
      sender: sender as `0x${string}`,
      ucs03: UNION_UCS03,
    })
  );
  console.log(JSON.stringify(sendBackCallEffectPayload));
  // ========================================================================

  // Calls
  const callsPayload = [
    bondPayload,
    allowancePayload,
    sendBackCallEffectPayload,
  ];

  const calls = callInstruction(
    sender.toLowerCase(),
    toHex(proxy_address),
    Schema.decodeSync(HexFromJson)(callsPayload)
  );

  return calls;
};

export const getBabylonCallsInstruction = async (
  sender: string,
  amount: string,
  min_mint_amount: bigint,
  proxy_address: string,
  channel_id: number,
  mainnet: boolean
) => {
  let lst_address = mainnet
    ? BABYLON_MAINNET_LST_CONTRACT
    : BABYLON_LST_CONTRACT;
  let bondPayload = createBabylonBondPayload({
    lst_address,
    mint_to_address: proxy_address,
    amount,
    min_mint_amount: min_mint_amount.toString(),
    denom: "ubbn",
  });

  console.log(JSON.stringify(bondPayload));

  let ebabyOnBabylon = mainnet
    ? EBABY_ON_BABYLON_MAINNET
    : EBABY_ON_BABYLON_TESTNET;

  let allowancePayload = createIncreaseAllowance(
    BABYLON_ZKGM_MINTER_ADDRESS,
    min_mint_amount.toString(),
    ebabyOnBabylon
  );

  console.log(JSON.stringify(allowancePayload));

  const sendBackCallEffectPayload = await Effect.runPromise(
    getBabylonSendbackCallMsg({
      baseToken: ebabyOnBabylon,
      channel_id: channel_id as number & Brand<"ChannelId">,
      metadata: toHex(""),
      minAmount: min_mint_amount,
      quoteToken: EBABY_ERC20_SEPOLIA as `0x${string}`,
      receiver: proxy_address,
      sender: sender as `0x${string}`,
      ucs03: BABYLON_UCS03,
    })
  );
  console.log(JSON.stringify(sendBackCallEffectPayload));

  // Calls
  const callsPayload = [
    bondPayload,
    allowancePayload,
    sendBackCallEffectPayload,
  ];

  const calls = callInstruction(
    sender.toLowerCase(),
    toHex(proxy_address),
    Schema.decodeSync(HexFromJson)(callsPayload)
  );

  return calls;
};

export const getBabylonUnbondCallsInstruction = async (
  sender: string,
  amount: string,
  channel_id: number,
  proxy_address: string
) => {
  // give allowance to lst contract to transfer ebaby from proxy contract
  let allowancePayload = createIncreaseAllowance(
    BABYLON_LST_CONTRACT,
    amount,
    EBABY_ON_BABYLON_MAINNET
  );

  console.log(JSON.stringify(allowancePayload));

  // call unbond to lst contract
  let unbondPayload = createBabylonUnbondPayload({
    lst_address: BABYLON_LST_CONTRACT,
    recipient_address: sender,
    amount,
    channel_id,
  });

  // Calls
  const callsPayload = [allowancePayload, unbondPayload];

  const calls = callInstruction(
    sender.toLowerCase(),
    toHex(proxy_address),
    Schema.decodeSync(HexFromJson)(callsPayload)
  );

  return calls;
};

export const bondFromHoleskyToUnion = async (
  signer: ethers.Wallet,
  amount: bigint,
  proxy_address: string
) => {
  let salt = getSalt();
  console.log(salt);

  //approve ucs03 contract to transfer first
  const erc20Contract = new ethers.Contract(U_ERC20, erc20Abi, signer);
  const resp = await erc20Contract.approve(ETH_UCS03, amount);
  console.log(resp);

  let txReceipt = await resp.wait();
  console.log(txReceipt);

  let sender = await signer.getAddress();

  let rate = await fetchExchangeRate("purchase_rate");
  let min_mint_amount = BigInt(Math.floor(Number(amount) * Number(rate)));

  let tokenOrder = tokenOrderV2(
    sender.toLowerCase(),
    proxy_address,
    U_ERC20,
    amount,
    AU_ON_UNION_QUOTE_TOKEN,
    amount
  );

  let calls = await getUnionCallsInstruction(
    sender,
    amount.toString(),
    min_mint_amount,
    proxy_address,
    UNION_TO_HOLESKY_CHANNEL_ID
  );

  console.log({ tokenOrder, calls });
  // // Batch Call
  const batchCall: Batch = getInstructionBatch([tokenOrder, calls]);

  //console.log({ tokenOrder, calls });
  console.log({ batchCall });

  const batchInstructions: [
    { version: number; opcode: number; operand: `0x${string}` }[]
  ] = [
    [
      // Tokenorder, send U token
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
  //console.log({ batchInstructions, batchOperand });

  const ucs03Contract = new ethers.Contract(ETH_UCS03, ucs03abi, signer);

  const transferAndCallRes = await ucs03Contract.send(
    HOLESKY_TO_UNION_CHANNEL_ID,
    0, //eureka set to false
    getTimeoutInNanoseconds7DaysFromNow(),
    salt,
    {
      version: batchCall.version,
      opcode: batchCall.opcode,
      operand: batchOperand,
    },
    { gasLimit: 500000 } // Adjust gas limit as needed
  );

  console.log(transferAndCallRes);
  let transferAndCallReceipt = await transferAndCallRes.wait();
  console.log(transferAndCallReceipt);
};

export const bondFromEthereumToBabylon = async (
  signer: ethers.Wallet,
  ethChainName: string,
  amount: bigint,
  channel_id: number,
  proxy_address: string
) => {
  let salt = getSalt();
  console.log(salt);

  //approve ucs03 contract to transfer first
  const erc20Contract = new ethers.Contract(
    BABY_ERC20[ethChainName],
    erc20Abi,
    signer
  );
  const resp = await erc20Contract.approve(ETH_UCS03, amount);
  console.log(resp);

  let txReceipt = await resp.wait();
  console.log(txReceipt);

  let sender = await signer.getAddress();

  let mainnet = ethChainName === "ethereum";
  let rate = await fetchBabylonExchangeRate("exchange_rate", mainnet);
  console.log("rate", rate);
  let min_mint_amount = BigInt(Math.floor(Number(amount) * Number(rate)));

  let tokenOrder = tokenOrderV2Unescrow(
    sender.toLowerCase(),
    proxy_address,
    BABY_ERC20[ethChainName],
    amount,
    BBN_ON_BABYLON_QUOTE_TOKEN,
    amount
  );

  let calls = await getBabylonCallsInstruction(
    sender,
    amount.toString(),
    min_mint_amount,
    proxy_address,
    BABYLON_SOURCE_CHANNEL_ID[ethChainName],
    mainnet
  );

  console.log({ tokenOrder, calls });
  // // Batch Call
  const batchCall: Batch = getInstructionBatch([tokenOrder, calls]);

  //console.log({ tokenOrder, calls });
  console.log({ batchCall });

  const batchInstructions: [
    { version: number; opcode: number; operand: `0x${string}` }[]
  ] = [
    [
      // Tokenorder, send U token
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
  //console.log({ batchInstructions, batchOperand });

  const ucs03Contract = new ethers.Contract(ETH_UCS03, ucs03abi, signer);

  const transferAndCallRes = await ucs03Contract.send(
    channel_id,
    0, //eureka set to false
    getTimeoutInNanoseconds7DaysFromNow(),
    salt,
    {
      version: batchCall.version,
      opcode: batchCall.opcode,
      operand: batchOperand,
    },
    { gasLimit: 500000 } // Adjust gas limit as needed
  );

  console.log(transferAndCallRes);
  let transferAndCallReceipt = await transferAndCallRes.wait();
  console.log(transferAndCallReceipt);
};

export const unbondFromSepoliaToBabylon = async (
  signer: ethers.Wallet,
  amount: bigint,
  channel_id: number,
  proxy_address: string
) => {
  let salt = getSalt();
  console.log(salt);

  const erc20Contract = new ethers.Contract(
    EBABY_ERC20_SEPOLIA,
    erc20Abi,
    signer
  );
  const resp = await erc20Contract.approve(ETH_UCS03, amount);
  console.log(resp);

  let txReceipt = await resp.wait();
  console.log(txReceipt);

  let sender = await signer.getAddress();

  console.log("amount:", amount);

  let tokenOrder = tokenOrderV2Unescrow(
    sender.toLowerCase(),
    proxy_address,
    EBABY_ERC20_SEPOLIA,
    amount,
    EBABY_ON_BABYLON_QUOTE_TOKEN,
    amount
  );

  let calls = await getBabylonUnbondCallsInstruction(
    sender,
    amount.toString(),
    BABYLON_TO_SEPOLIA_CHANNEL_ID,
    proxy_address
  );

  console.log({ tokenOrder, calls });
  // // Batch Call
  const batchCall: Batch = getInstructionBatch([tokenOrder, calls]);

  //console.log({ tokenOrder, calls });
  console.log({ batchCall });

  const batchInstructions: [
    { version: number; opcode: number; operand: `0x${string}` }[]
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
  //console.log({ batchInstructions, batchOperand });

  const ucs03Contract = new ethers.Contract(ETH_UCS03, ucs03abi, signer);

  const transferAndCallRes = await ucs03Contract.send(
    channel_id,
    0, //eureka set to false
    getTimeoutInNanoseconds7DaysFromNow(),
    salt,
    {
      version: batchCall.version,
      opcode: batchCall.opcode,
      operand: batchOperand,
    },
    { gasLimit: 500000 } // Adjust gas limit as needed
  );

  console.log(transferAndCallRes);
  let transferAndCallReceipt = await transferAndCallRes.wait();
  console.log(transferAndCallReceipt);
};

export const unbondFromHoleskyToBabylon = async (
  signer: ethers.Wallet,
  amount: bigint,
  channel_id: number,
  proxy_address: string
) => {
  let salt = getSalt();
  console.log(salt);

  const erc20Contract = new ethers.Contract(
    EBABY_ERC20_HOLESKY,
    erc20Abi,
    signer
  );
  const resp = await erc20Contract.approve(ETH_UCS03, amount);
  console.log(resp);

  let txReceipt = await resp.wait();
  console.log(txReceipt);

  let sender = await signer.getAddress();

  console.log("amount:", amount);

  let tokenOrder = tokenOrderV2Unescrow(
    sender.toLowerCase(),
    proxy_address,
    EBABY_ERC20_HOLESKY,
    amount,
    EBABY_ON_BABYLON_QUOTE_TOKEN,
    amount
  );

  let calls = await getBabylonUnbondCallsInstruction(
    sender,
    amount.toString(),
    BABYLON_TO_HOLESKY_CHANNEL_ID,
    proxy_address
  );

  console.log({ tokenOrder, calls });
  // // Batch Call
  const batchCall: Batch = getInstructionBatch([tokenOrder, calls]);

  //console.log({ tokenOrder, calls });
  console.log({ batchCall });

  const batchInstructions: [
    { version: number; opcode: number; operand: `0x${string}` }[]
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
  //console.log({ batchInstructions, batchOperand });

  const ucs03Contract = new ethers.Contract(ETH_UCS03, ucs03abi, signer);

  const transferAndCallRes = await ucs03Contract.send(
    channel_id,
    0, //eureka set to false
    getTimeoutInNanoseconds7DaysFromNow(),
    salt,
    {
      version: batchCall.version,
      opcode: batchCall.opcode,
      operand: batchOperand,
    },
    { gasLimit: 500000 } // Adjust gas limit as needed
  );

  console.log(transferAndCallRes);
  let transferAndCallReceipt = await transferAndCallRes.wait();
  console.log(transferAndCallReceipt);
};

export const fetchExchangeRate = async (rate: string) => {
  try {
    const client = await CosmWasmClient.connect(UNION_RPC_URL);
    const response = await client.queryContractSmart(UNION_LST_CONTRACT, {
      accounting_state: {},
    });
    return response[rate];
  } catch (e) {
    console.error(e);
  }
};

export const fetchBabylonExchangeRate = async (
  rate: string,
  mainnet: boolean
) => {
  let babylonRpcURL = mainnet ? BABYLON_MAINNET_RPC_URL : BABYLON_RPC_URL;
  let babylonLstContract = mainnet
    ? BABYLON_MAINNET_LST_CONTRACT
    : BABYLON_LST_CONTRACT;

  try {
    const client = await CosmWasmClient.connect(babylonRpcURL);
    const response = await client.queryContractSmart(babylonLstContract, {
      staking_liquidity: {},
    });
    return response[rate];
  } catch (e) {
    console.error(e);
  }
};
