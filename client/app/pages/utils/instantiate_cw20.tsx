"use client";

import {
  Card,
  CardBody,
  CardFooter,
  Button,
  Input,
} from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";

export default function InstantiateCW20() {
  const { userAddress, client, network } = useGlobalContext();

  const cw20CodeId = network.toString() === "uniontestnet" ? 252 : 192;

  const handleSubmit = async (e: any) => {
    // Prevent the browser from reloading the page
    e.preventDefault();
    if (!userAddress) {
      alert("no user");
      return;
    }
    const form = e.target;
    const formData = new FormData(form);
    const unionMsg = {
      decimals: 6,
      name: "emuno",
      symbol: "eUNO",
      initial_balances: [],
      mint: {
        minter: userAddress
      }
    };

    const babyMsg = {
      decimals: 6,
      name: "ebbn",
      symbol: "eBABY",
      initial_balances: [],
      mint: {
        minter: userAddress
      }
    };

    const msg = network.toString() === "uniontestnet" ? unionMsg : babyMsg;
    const formJson = Object.fromEntries(formData.entries());
    const code_id = formJson.code_id;
    console.log(JSON.stringify(msg));
    console.log(code_id);

    try {
      const instantiateOptions = {
        memo: "Instantiating a new contract",
        funds: [],
        admin: userAddress,
      };

      const instantiateResult = await client?.instantiate(
        userAddress,
        Number(code_id),
        msg,
        "ebaby",
        "auto",
        instantiateOptions
      );
      console.log(instantiateResult?.transactionHash);
      console.log(instantiateResult?.contractAddress);
      alert(instantiateResult?.contractAddress);

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
              name="code_id"
              label="CW20 CodeID"
              className="max-w-xs"
              defaultValue={cw20CodeId.toString()}
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
