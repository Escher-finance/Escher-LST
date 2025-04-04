"use client";

import {
  Card,
  CardBody,
  CardFooter,
  Button,
  Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";

export default function SetAdmin() {
  const { userAddress, client, network } = useGlobalContext();

  const handleSubmit = async (e: any) => {
    // Prevent the browser from reloading the page
    e.preventDefault();
    if (!userAddress) {
      return;
    }
    const form = e.target;
    const formData = new FormData(form);
    const formEntries = Object.fromEntries(formData.entries());
    const denom = "actory/union1vnglhewf3w66cquy6hr7urjv3589srheampz42/limuno";
    const lst_contract = formEntries.lst_contract.toString();
    const msg = {
      set_token_admin: {
        denom,
        new_admin: "union1vnglhewf3w66cquy6hr7urjv3589srheampz42"
      }
    };

    console.log(JSON.stringify(msg));
    try {
      const res = await client?.execute(userAddress, lst_contract, msg, "auto", "execute bond", []);
      alert(res?.transactionHash);

    } catch (err) {
      console.log(err);
    }
  };

  return (
    <div className="w-full flex">
      <form onSubmit={handleSubmit} className="w-full flex">
        <Card className="grow">
          <CardBody className="w-full gap-4">
            <Input
              isRequired
              name="lst_contract"
              label="LiquidStaking Contract"
              defaultValue="union1tkqqlr3xdvr20ywnmtjgrdstqdepj74teq5vmltgl2zvsn9sgwusrdfv8y"
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