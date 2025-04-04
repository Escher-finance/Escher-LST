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
import { getSalt } from "@/app/lib/salt";

interface KeyProps {
  stateKey: number;
  setStateKey: (key: number) => void;
}

export default function ExecuteBond({ stateKey, setStateKey }: KeyProps) {
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

    const expected = Math.floor(Number(amount) / liquidity.exchange_rate);

    const msg = {
      bond: {
        salt: getSalt(),
        expected: expected.toString(),
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
