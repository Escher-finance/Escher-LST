"use client";

import {
  Card,
  CardBody,
  CardFooter,
  Button,
  Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";
import { useState } from "react";
import { getSalt, toHex } from "@/app/lib/salt";

interface KeyProps {
  stateKey: number;
  setStateKey: (key: number) => void;
}

export default function Bond({ stateKey, setStateKey }: KeyProps) {
  const { userAddress, client, network } = useGlobalContext();

  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = async (e: any) => {

    e.preventDefault();
    if (!userAddress) {
      alert("no user");
      return;
    }


    const liquidity = await client?.queryContractSmart(
      network?.contracts.lst,
      {
        staking_liquidity: {}
      }
    );



    const form = e.target;
    const formData = new FormData(form);
    const formEntries = Object.fromEntries(formData.entries());
    const amount = formEntries.amount.toString();
    const recipient = formEntries.recipient.toString();
    const recipient_channel_id = formEntries.recipient_channel_id.toString();

    const expected = Math.floor(Number(amount) / liquidity.exchange_rate);
    const encoder = new TextEncoder();
    const recipient_hex = toHex(encoder.encode(recipient));


    // recipient: recipient == "" ? null : recipient.indexOf("bbn") != -1 ? recipient : recipient_hex,
    // recipient_channel_id: recipient_channel_id == "0" ? null : Number(recipient_channel_id),

    const msg = {
      bond: {
        salt: getSalt(),
        expected: expected.toString(),
        recipient: recipient == "" ? null : recipient.indexOf("bbn") != -1 ? recipient : recipient_hex,
        recipient_channel_id: recipient_channel_id == "0" ? null : Number(recipient_channel_id),

      },
    };

    if (Number(amount) < 1000) {
      alert("Sorry, minimal bond amount is 1000000");
      return;
    }

    console.log(JSON.stringify(msg));
    try {
      setIsLoading(true);
      const funds = [{
        amount,
        denom: network?.stakeCurrency.coinMinimalDenom
      }];
      const res = await client?.execute(userAddress, network?.contracts.lst, msg, "auto", "execute bond", funds);
      alert(res?.transactionHash);
      let newKey = stateKey + 1;
      setStateKey(newKey);
      setIsLoading(false);
    } catch (err) {
      console.log(err);
      setIsLoading(false);
    }
  };

  return (
    <div className="w-full flex flex-row gap-4">
      <form onSubmit={handleSubmit} className="w-full flex">
        <Card className="w-full flex">
          <CardBody className="gap-4">
            <Input
              isRequired
              name="amount"
              label="Amount"
              defaultValue="10000"
            />
            <Input
              name="recipient"
              label="Recipient"
              defaultValue=""
            />
            <Input
              name="recipient_channel_id"
              label="Recipient Channel ID (4 for xion)"
              defaultValue="0"
            />
          </CardBody>
          <CardFooter>
            <Button type="submit" isLoading={isLoading}>Bond</Button>
          </CardFooter>
        </Card>
      </form>
    </div>
  );
}

//bbn1fh0yyvuxz7l0vcusq5jc9zvzpm8ec2auvvkh44
//xion1vnglhewf3w66cquy6hr7urjv3589srhe496gds