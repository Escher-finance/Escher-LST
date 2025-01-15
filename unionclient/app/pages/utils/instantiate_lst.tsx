"use client";
import { v4 as uuidv4 } from 'uuid';

import {
  Card,
  CardBody,
  CardFooter,
  Button,
  Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";

export default function InstantiateLiquidStaking() {
  const { userAddress, client, network } = useGlobalContext();

  const handleSubmit = async (e: any) => {
    // Prevent the browser from reloading the page
    e.preventDefault();
    if (!userAddress) {
      alert("no user");
      return;
    }
    const form = e.target;
    const formData = new FormData(form);
    const formEntries = Object.fromEntries(formData.entries());
    const liquid_staking_code_id = Number(formEntries.liquid_staking_code_id);


    const msg = {
      underlying_coin_denom: "muno",
      validators: [
        { weight: 1, address: "unionvaloper1q20xpxw32wmrkm97ha6klj3hqpl4e22jxuqjju" }
      ],
      liquidstaking_denom: "limuno",
      ucs03_channel: "channel-86",
      ucs03_relay_contract: "union1m87a5scxnnk83wfwapxlufzm58qe2v65985exff70z95a2yr86yq7hl08h",
      fee_rate: "0.1",
      revenue_receiver: "union1vnglhewf3w66cquy6hr7urjv3589srheampz42",
      reward_code_id: 40,
      unbonding_time: 60,
      salt: uuidv4(),
    };

    console.log(JSON.stringify(msg));
    try {
      const instantiateOptions = {
        memo: "Instantiating a new contract",
        funds: [],
        admin: userAddress,
      };

      const instantiateResult = await client?.instantiate(
        userAddress,
        liquid_staking_code_id,
        msg,
        "lst",
        "auto",
        instantiateOptions
      );
      console.log(instantiateResult);

    } catch (err) {
      console.log(err);
    }
  };

  return (
    <div className="w-full flex flex-row gap-4">
      <form onSubmit={handleSubmit}>
        <Card>
          <CardBody className="gap-4">
            <Input
              isRequired
              name="liquid_staking_code_id"
              label="Liquid Staking CodeID"
              className="max-w-xs"
              defaultValue="41"
            />
          </CardBody>
          <CardFooter>
            <Button type="submit">Submit</Button>
          </CardFooter>
        </Card>
      </form>
    </div>
  );
}
