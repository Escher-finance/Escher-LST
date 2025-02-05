"use client";

import {
  Card,
  CardBody,
  CardFooter,
  Button,
  Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";
import { getExecuteContractMessage } from "@/utils/msg";
import { useState } from "react";

interface KeyProps {
  stateKey: number;
  setStateKey: (key: number) => void;
}

export default function Unbond({ stateKey, setStateKey }: KeyProps) {
  const { userAddress, client, network } = useGlobalContext();
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = async (e: any) => {
    // Prevent the browser from reloading the page
    e.preventDefault();
    setIsLoading(true);
    const form = e.target;
    const formData = new FormData(form);
    const formEntries = Object.fromEntries(formData.entries());
    const amount = formEntries.amount.toString();

    const msg: any = {
      staking_liquidity: {}
    };


    const liquidity = await client?.queryContractSmart(
      network?.contracts.lst,
      msg
    );

    let undelegate_amount = Number(amount) * liquidity.exchange_rate;

    let max_amount = Math.floor(liquidity.delegated / liquidity.exchange_rate);
    if (undelegate_amount >= liquidity.delegated) {
      alert("Not enough fund to be undelegated, please reduce your unbonding amount to below < " + max_amount.toString());
      setIsLoading(false);
      return;
    }

    try {
      if (!userAddress) {
        alert("no user wallet");
        setIsLoading(false);
        return;
      }

      const transferCW20TokenMsg = {
        transfer: {
          recipient: network?.contracts.lst,
          amount
        },
      };
      const funds = [
        {
          denom: network?.stakeCurrency.liquidStakingDenom,
          amount
        }
      ];
      const unbondingMsg = {
        unbond: {
          staker: userAddress,
        }
      };
      console.log(JSON.stringify(unbondingMsg));
      const executeUnbondingMsg = getExecuteContractMessage(userAddress, network?.contracts.lst, unbondingMsg, funds);


      let msgs = [executeUnbondingMsg];
      const res = await client?.signAndBroadcast(userAddress, msgs, "auto", "");
      alert(res?.transactionHash);
      console.log(res?.transactionHash);

      let newKey = stateKey + 1;
      setStateKey(newKey);
      setIsLoading(false);
    } catch (err) {
      setIsLoading(false);
      console.log(err);
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
            <Button type="submit" isLoading={isLoading}>Unbond</Button>
          </CardFooter>
        </Card>
      </form>
    </div>
  );
}

