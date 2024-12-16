"use client";

import React, {FormEventHandler} from "react";
import {AxelarQueryAPI, CHAINS, Environment, GasToken} from "@axelar-network/axelarjs-sdk";
import {Card, CardBody, CardFooter, Button, Input, Select, SelectItem} from "@nextui-org/react";
import {useGlobalContext} from "../core/context";

const axelarFees = async (args: {target: string; source: string; tokenFee: string}) => {
  const {target, source, tokenFee} = args;

  const axelarQuery = new AxelarQueryAPI({
    environment: Environment.MAINNET,
  });

  const fee = await axelarQuery.estimateGasFee(
    CHAINS.MAINNET.ARCHWAY,
    CHAINS.MAINNET.NEUTRON,
    GasToken.AXL,
    250000,
    undefined,
    undefined,
    {
      showDetailedFees: false,
      destinationContractAddress: target,
      sourceContractAddress: source,
      tokenSymbol: tokenFee,
    }
  );

  return fee as string;
};

export default function SendAxelar() {
  const {userAddress, client, network, voteAddress, walletAddress} = useGlobalContext();
  const networks = Object.values(CHAINS.MAINNET);

  const transformMessage = (message: string) => {
    const encoder = new TextEncoder();
    const jsonString = JSON.stringify({
      receive_message_cosmos: {
        sender: userAddress,
        message,
      },
    });
    const payload = [0, 0, 0, 2].concat(Array.from(encoder.encode(jsonString)));

    return payload;
  };

  const handleSubmit: FormEventHandler = async (e) => {
    try {
      e.preventDefault();
      const form: any = e.target;
      const formData = new FormData(form);
      const formJson = Object.fromEntries(formData.entries());
      const relayer = "archway14q6tpqne0k92qdytuzf7pctf7qmmk89mlfe3qmf53m7dku2t4qwqj0lxe9";
      const axelarGateway = "axelar1dv4u5k73pzqrxlzujxg3qp8kvc3pje7jtdvu72npnt5zhq05ejcsn5qme5";

      const fee = await axelarFees({
        source: walletAddress as string,
        target: formJson.destinationAddress as string,
        tokenFee: "axl",
      });
      const relayerMessage = {
        multichain_message: {
          destination_chain: formJson.destinationChain,
          destination_address: formJson.destinationAddress,
          source_channel: "channel-13",
          message: transformMessage("FROM CLIENT NEWWW"),
          fee: {amount: fee, recipient: "axelar1aythygn6z5thymj6tmzfwekzh05ewg3l7d6y89"},
        },
      };
      const message = {
        wasm: {
          execute: {
            contract_addr: relayer,
            msg: Buffer.from(JSON.stringify(relayerMessage)).toString("base64"),
          },
        },
      };

      const {transactionHash} = await client.execute(
        userAddress as string,
        walletAddress,
        {
          propose: {
            title: "multichain relayer",
            description: "example message",
            msgs: [message],
          },
        },
        "auto"
      );
      console.log("Hash: ", transactionHash);
    } catch (err) {
      console.log(err);
    }
  };

  return (
    <div className="w-full gap-4 px-2 py-2">
      <form onSubmit={handleSubmit}>
        <Card>
          <CardBody className="gap-4">
            <Input
              isRequired
              name="destinationAddress"
              label="Destination Address"
              className="max-w-xs"
            />
            <Input
              isRequired
              name="destinationChain"
              label="Destination Chain"
              className="max-w-xs"
            />
            <Input isRequired name="message" label="Message" className="max-w-xs" />
            <Input isRequired name="sourceChannel" label="Source Channel" className="max-w-xs" />
          </CardBody>
          <CardFooter>
            <Button type="submit">Send</Button>
          </CardFooter>
        </Card>
      </form>
    </div>
  );
}
