"use client";

import {
  Card,
  CardBody,
  CardFooter,
  Button,
  Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";

export default function SetParams() {
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
    const liquidstaking_denom = formEntries.liquidstaking_denom.toString();
    const lst_contract = formEntries.lst_contract.toString();
    const msg = {
      set_parameters: {
        liquidstaking_denom,
      }
    };

    console.log(JSON.stringify(msg));
    try {
      const res = await client?.execute(userAddress, lst_contract, msg, "auto");
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
              defaultValue="union1x70fmdv965fj6hm4lmyudxyphl6j9vweukmc3fxja3mamgqrup6qf9mv3x"
            />
            <Input
              isRequired
              name="liquidstaking_denom"
              label="LiquidStaking Denom"
              defaultValue="factory/union1vnglhewf3w66cquy6hr7urjv3589srheampz42/lmuno"
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