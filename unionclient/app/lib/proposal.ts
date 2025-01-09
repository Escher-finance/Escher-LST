import { EncodeObject, Registry } from "@cosmjs/proto-signing";
import { defaultRegistryTypes } from "@cosmjs/stargate";
import { wasmTypes } from "@cosmjs/cosmwasm-stargate";
import Long from "long";
import { osmosisProtoRegistry } from "@osmosis-labs/proto-codecs";
import BigNumber from "bignumber.js";

const allTypes = [
  ...defaultRegistryTypes,
  ...wasmTypes,
  ...osmosisProtoRegistry,
];
const registry = new Registry(allTypes);

export const createTransferTokenMsg = ({
  icaAddress,
  denom,
  amount,
  toAddress,
}: {
  icaAddress: string;
  denom: string;
  amount: string;
  toAddress: string;
}) => {
  const typeUrl = "/cosmos.bank.v1beta1.MsgSend";
  return {
    typeUrl,
    value: {
      fromAddress: icaAddress, // need to find out how to get the ica address
      amount: [
        {
          denom,
          amount,
        },
      ],
      toAddress,
    },
  };
};

export const createDelegateStakeMsg = ({
  denom,
  amount,
  delegatorAddress,
  validatorAddress,
}: {
  denom: string;
  amount: string;
  delegatorAddress: string;
  validatorAddress: string;
}) => {
  const typeUrl = "/cosmos.staking.v1beta1.MsgDelegate";
  return {
    typeUrl,
    value: {
      delegatorAddress,
      validatorAddress,
      amount: {
        denom,
        amount,
      },
    },
  };
};

export const createUndelegateStakeMsg = ({
  denom,
  amount,
  delegatorAddress,
  validatorAddress,
}: {
  denom: string;
  amount: string;
  delegatorAddress: string;
  validatorAddress: string;
}) => {
  const typeUrl = "/cosmos.staking.v1beta1.MsgUndelegate";
  return {
    typeUrl,
    value: {
      delegatorAddress,
      validatorAddress,
      amount: {
        denom,
        amount,
      },
    },
  };
};

export const createSendIBCMsg = ({
  sender,
  denom,
  amount,
  sourceChannel,
  receiver,
  timeoutTimestamp,
}: {
  sender: string;
  denom: string;
  amount: string;
  sourceChannel: string;
  receiver: string;
  timeoutTimestamp: Long;
}) => {
  const typeUrl = "/ibc.applications.transfer.v1.MsgTransfer";
  return {
    typeUrl,
    value: {
      sourcePort: "transfer",
      sourceChannel,
      token: {
        amount,
        denom,
      },
      sender,
      receiver,
      timeoutTimestamp,
      memo: "Send IBC",
    },
  };
};

interface SwapRoute {
  poolId: string;
  tokenOutDenom: string;
}

export const createOsmosisSwapMsg = ({
  routes,
  sender,
  amount,
  denom,
  tokenOutMinAmount,
}: {
  routes: SwapRoute[];
  sender: string;
  amount: string;
  denom: string;
  tokenOutMinAmount: string;
}) => {
  const msgObj = {
    routes,
    sender,
    tokenIn: {
      amount,
      denom,
    },
    tokenOutMinAmount,
  };

  const sendMsgTypeURL = "/osmosis.poolmanager.v1beta1.MsgSwapExactAmountIn";

  return {
    typeUrl: sendMsgTypeURL,
    value: msgObj,
  };
};

export const createIBCTransferMsg = ({channel_id, to_address, amount, timestamp } : {
  channel_id: string, to_address: string, amount: any, timestamp: string}) => {
  return {
        ibc: {
          transfer: {
            channel_id,
            to_address,
            amount,
            timeout: {
              timestamp,
            },
          },
        },
      }
    ;
};

export const getControllerMessage = (msg: EncodeObject) => {
  let encodedMsg = registry.encode(msg);
  const controllerMessage = {
    send_cosmos_msgs: {
      messages: [
        {
          stargate: {
            type_url: msg.typeUrl,
            value: Buffer.from(encodedMsg).toString("base64"),
          },
        },
      ],
      packet_memo: "packet memo by nomos",
    },
  };

  return controllerMessage;
};

export const getControllerMessageWithMemo = (msg: EncodeObject, packet_memo: string|undefined) => {
  let encodedMsg = registry.encode(msg);
  const controllerMessage = {
    send_cosmos_msgs: {
      messages: [
        {
          stargate: {
            type_url: msg.typeUrl,
            value: Buffer.from(encodedMsg).toString("base64"),
          },
        },
      ],
      packet_memo: packet_memo? packet_memo : "packet memo by nomos",
    },
  };

  return controllerMessage;
};

const getStargateMsg = (msg: EncodeObject) => {
  console.log("encode msg", JSON.stringify(msg));
  let encodedMsg = registry.encode(msg);
  return {
    stargate: {
      type_url: msg.typeUrl,
      value: Buffer.from(encodedMsg).toString("base64"),
    },
  };
};

const getBatchControllerMessage = (msgs: EncodeObject[]) => {
  let cosmos_msgs: any[] = [];
  msgs.forEach((msg) => {
    const stargateMsg = getStargateMsg(msg);
    cosmos_msgs.push(stargateMsg);
  });

  const controllerMessage = {
    send_cosmos_msgs: {
      messages: cosmos_msgs,
      packet_memo: "packet memo by nomos",
    },
  };

  return controllerMessage;
};

export const produceICAProposal = (
  title: string,
  description: string,
  msg: EncodeObject,
  icaControllerAddress: string | null,
) => {
  const controllerMessage = getControllerMessage(msg);
  return {
    title,
    description,
    msgs: [
      {
        wasm: {
          execute: {
            contract_addr: icaControllerAddress,
            msg: Buffer.from(JSON.stringify(controllerMessage)).toString(
              "base64"
            ),
            funds: [],
          },
        },
      },
    ],
  };
};


export const produceICAProposalWithMemo = (
  title: string,
  description: string,
  msg: EncodeObject,
  icaControllerAddress: string | null,
  memo: string | undefined
) => {
  const controllerMessage = getControllerMessageWithMemo(msg, memo);
  return {
    title,
    description,
    msgs: [
      {
        wasm: {
          execute: {
            contract_addr: icaControllerAddress,
            msg: Buffer.from(JSON.stringify(controllerMessage)).toString(
              "base64"
            ),
            funds: [],
          },
        },
      },
    ],
  };
};

export const createExecuteContractMsg = ({
  sender,
  contract,
  message,
}: {
  sender: string;
  contract: string;
  message: object,
}) => {
  const typeUrl = "/cosmwasm.wasm.v1.MsgExecuteContract";
  return {
    typeUrl,
    value: {
      sender,
      contract,
      msg: Buffer.from(JSON.stringify(message)).toString("base64"),
    }
  };
};


export async function createICAProposalWithMemo({
  title,
  description,
  client,
  userAddress,
  proposalPayload,
  voteAddress,
  icaControllerAddress,
  memo,
}: {
  title: string;
  description: string;
  client: any;
  userAddress: string | null;
  proposalPayload: any;
  voteAddress: string | null;
  icaControllerAddress: string | null;
  memo: string | undefined;
}) {
  const proposalMsg = {
    propose: produceICAProposalWithMemo(
      title,
      description,
      proposalPayload,
      icaControllerAddress,
      memo
    ),
  };
  console.log("proposalMsg", JSON.stringify(proposalMsg));

  try {
    const executionResponse = await client?.execute(
      userAddress,
      voteAddress,
      proposalMsg,
      "auto"
    );
    console.log("executionResponse", executionResponse);

    const proposal_id = executionResponse?.events
      .find((e: any) => e.type === "wasm")
      ?.attributes.find((a: any) => a.key === "proposal_id")?.value;
    console.log("proposal_id", proposal_id);
    return { proposal_id };
  } catch (error) {
    console.log("error", error);
    alert(error);
    return { proposal_id: "null" };
  }
}


export async function createProposal({
  title,
  description,
  client,
  userAddress,
  proposalPayload,
  voteAddress,
  icaControllerAddress,
}: {
  title: string;
  description: string;
  client: any;
  userAddress: string | null;
  proposalPayload: any;
  voteAddress: string | null;
  icaControllerAddress: string | null;
}) {
  const proposalMsg = {
    propose: produceICAProposal(
      title,
      description,
      proposalPayload,
      icaControllerAddress
    ),
  };

  console.log("proposalMsg", JSON.stringify(proposalMsg));

  try {
    const executionResponse = await client?.execute(
      userAddress,
      voteAddress,
      proposalMsg,
      "auto"
    );
    console.log("executionResponse", executionResponse);

    const proposal_id = executionResponse?.events
      .find((e: any) => e.type === "wasm")
      ?.attributes.find((a: any) => a.key === "proposal_id")?.value;
    console.log("proposal_id", proposal_id);
    return { proposal_id };
  } catch (error) {
    console.log("error", error);
    alert(error);
    return { proposal_id: "null" };
  }
}

export function buildWasmMsg({
  icaControllerAddress,
  message,
}: {
  icaControllerAddress: string;
  message: EncodeObject;
}) {
  let controllerMessage = getControllerMessage(message);
  let msg = {
    wasm: {
      execute: {
        contract_addr: icaControllerAddress,
        msg: Buffer.from(JSON.stringify(controllerMessage)).toString("base64"),
        funds: [],
      },
    },
  };

  return msg;
}

export function buildBatchICAWasmMsg({
  icaControllerAddress,
  messages,
}: {
  icaControllerAddress: string;
  messages: EncodeObject[];
}) {
  let batchControllerMessages: any = getBatchControllerMessage(messages);
  console.log(
    "batchControllerMessages",
    JSON.stringify(batchControllerMessages)
  );
  let msg = {
    wasm: {
      execute: {
        contract_addr: icaControllerAddress,
        msg: Buffer.from(JSON.stringify(batchControllerMessages)).toString(
          "base64"
        ),
        funds: [],
      },
    },
  };

  return msg;
}

export function buildBatchProposeMsgs(title: string, description: string, msgs: any[]) {
  const proposalPayload = {
    propose: {
      title,
      description,
      msgs,
    },
  };
  return proposalPayload;
}

export async function createBatchProposal({
  title,
  description,
  client,
  userAddress,
  msgs,
  voteAddress,
}: {
  title: string;
  description: string;
  client: any;
  userAddress: string | null;
  msgs: any[];
  voteAddress: string | null;
}) {
  const proposalPayload = buildBatchProposeMsgs(title, description, msgs);
  console.log("proposalPayload", JSON.stringify(proposalPayload));
  try {
    const executionResponse = await client?.execute(
      userAddress,
      voteAddress,
      proposalPayload,
      "auto"
    );
    console.log("executionResponse", executionResponse);
    const transactionHash = executionResponse?.transactionHash;
    const height = executionResponse.height;
    const proposal_id = executionResponse?.events
      .find((e: any) => e.type === "wasm")
      ?.attributes.find((a: any) => a.key === "proposal_id")?.value;

    console.log("proposal_id", proposal_id);
    return { transactionHash, proposal_id, height };
  } catch (error) {
    console.log("error", error);
    alert(error);
    return { transactionHash: "null", proposal_id: "null", height: 0 };
  }
}

export async function createProposalwithBatchICAMessages({
  title,
  description,
  client,
  userAddress,
  payloads,
  voteAddress,
  icaControllerAddress,
}: {
  title: string;
  description: string;
  client: any;
  userAddress: string | null;
  payloads: any[];
  voteAddress: string | null;
  icaControllerAddress: string;
}) {
  let msgs: any[] = [];
  console.log("payloads", JSON.stringify(payloads));

  let batchIcaWasmMsg = buildBatchICAWasmMsg({
    icaControllerAddress,
    messages: payloads,
  });
  msgs.push(batchIcaWasmMsg);

  console.log("msgs", JSON.stringify(msgs));
  const proposalPayload = {
    propose: {
      title,
      description,
      msgs,
    },
  };

  try {
    const executionResponse = await client?.execute(
      userAddress,
      voteAddress,
      proposalPayload,
      "auto"
    );
    console.log("executionResponse", executionResponse);

    const proposal_id = executionResponse?.events
      .find((e: any) => e.type === "wasm")
      ?.attributes.find((a: any) => a.key === "proposal_id")?.value;
    console.log("proposal_id", proposal_id);
    return { proposal_id };
  } catch (error) {
    console.log("error", error);
    alert(error);
    return { proposal_id: "null" };
  }
}



export function createSendBalanceQueryWithCallback(
  targetAddress: string,
  denom: string,
  callback_address: string,
  ica_controller_address: string,
  ica_transfer_channel_id: string
) {
  let balance_query = {
    address: targetAddress,
    denom,
    callback_address,
    ica_controller_address,
    ica_transfer_channel_id,
  };

  const msg = {
    send_balance_query: balance_query,
  };
  return msg;
}

export function createExecuteContractWasmMsg({
  target_contract,
  message,
}: {
  target_contract: string;
  message: object;
}) {
  let msg = {
    wasm: {
      execute: {
        contract_addr: target_contract,
        msg: Buffer.from(JSON.stringify(message)).toString("base64"),
        funds: [],
      },
    },
  };

  return msg;
}

export const getSwapMessage = (
  pool_id: string,
  icaAddress: string,
  tokenInDenom: string,
  sendAmount: string,
  tokenOutDenom: string,
  tokenOutMinAmount: string,
  denomDecimal: number
) => {
  let coinDecimals: BigNumber = new BigNumber(
    Math.pow(10, denomDecimal)
  );
  let amount = new BigNumber(sendAmount)
    .multipliedBy(coinDecimals)
    .toString();

  const routes = [
    {
      poolId: pool_id,
      tokenOutDenom,
    },
  ];

  const sender = icaAddress ? icaAddress : "";
  const osmosisSwapPayload = createOsmosisSwapMsg({
    routes,
    sender,
    amount,
    denom: tokenInDenom,
    tokenOutMinAmount,
  });
  console.log(
    "Osmosis Swap proposalPayload",
    JSON.stringify(osmosisSwapPayload)
  );
  return osmosisSwapPayload;
};