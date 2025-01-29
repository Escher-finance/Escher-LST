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
        { weight: 1, address: "unionvaloper1qcyu42wvmw07rsnm9jn2k5dusdwvsu5g74kw54" }
      ],
      liquidstaking_denom: "factory/union1vnglhewf3w66cquy6hr7urjv3589srheampz42/limuno",
      ucs03_channel: 7,
      ucs03_relay_contract: "union19hspxmypfxsdsnxttma8rxvp7dtcmzhl9my0ee64avg358vlpawsdvucqa",
      fee_rate: "0.1",
      fee_receiver: "union17z2ea0dtzkpu9lc2eh0jcwxywh40th5e0xla5q",
      reward_code_id: 171,
      unbonding_time: 60,
      salt: uuidv4(),
      quote_token: "0xf2865969cf99a28bb77e25494fe12d5180fe0efd",
      lst_quote_token: "0x5Ab6dcad571D461b1b6562152EC8Dce51314a6D6"
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
              defaultValue="181"
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
