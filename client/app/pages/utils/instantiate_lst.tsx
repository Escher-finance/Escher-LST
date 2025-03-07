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

  const lstCodeId = network.toString() === "uniontestnet" ? 258 : 196;
  const rewardCodeId = network.toString() === "uniontestnet" ? 171 : 194;

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


    const unionMsg = {
      underlying_coin_denom: "muno",
      validators: [
        { weight: 1, address: "unionvaloper14qekdkj2nmmwea4ufg9n002a3pud23y87mnkjg" },
        { weight: 1, address: "unionvaloper13fx29mnt2ssfae5td4t9grqmdpg6dtr8pudva7" }
      ],
      liquidstaking_denom: "emuno",
      ucs03_relay_contract: "union1x2jzeup7uwfxjxxrtfna2ktcugltntgu6kvc0eeayk0d82l247cqz669ee",
      fee_rate: "0.1",
      fee_receiver: "union17z2ea0dtzkpu9lc2eh0jcwxywh40th5e0xla5q",
      reward_code_id: 171,
      unbonding_time: 60,
      salt: uuidv4(),
      cw20_address: "union1njygapqdpnkaz5m64rh8038p4uh3xty78ktcunzj946hc4ft7s6qeu0u3f",
      quote_tokens: [
        {
          channel_id: 1,
          quote_token: "0x55081d42b7381033ed1408608b510649bbe7464d",
          lst_quote_token: "0x6C709Bd3D3C27438DeE76816B42144500E75053c",
        }]
    };



    const babyMsg = {
      underlying_coin_denom: "ubbn",
      validators: [
        { weight: 1, address: "bbnvaloper109x4ruspxarwt62puwcenhclw36l9v7j92f0ex" },
      ],
      liquidstaking_denom: "ebbn",
      ucs03_relay_contract: "bbn144hnwjtykzje3r4eccszq33fegycymh680huylagm3tqrwxhjrjqvkul3y",
      fee_rate: "0.1",
      fee_receiver: "bbn17z2ea0dtzkpu9lc2eh0jcwxywh40th5ej00y9g",
      reward_code_id: rewardCodeId,
      unbonding_time: 1944000,
      salt: uuidv4(),
      cw20_address: "bbn144hnwjtykzje3r4eccszq33fegycymh680huylagm3tqrwxhjrjqvkul3y",
      quote_tokens: []
    };


    const msg = network.toString() === "uniontestnet" ? unionMsg : babyMsg;

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

      alert(instantiateResult?.contractAddress)

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
              defaultValue={lstCodeId.toString()}
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
