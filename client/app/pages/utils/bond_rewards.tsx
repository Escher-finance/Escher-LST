"use client";

import {
  Card,
  CardBody,
  CardFooter,
  Button,
  Input,
} from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";

export default function BondRewards() {
  const { userAddress, client } = useGlobalContext();

  const handleSubmit = async (e: any) => {
    // Prevent the browser from reloading the page
    e.preventDefault();
    if (!userAddress) {
      alert("no user is logged in");
      return;
    }
    const form = e.target;
    const formData = new FormData(form);
    const formEntries = Object.fromEntries(formData.entries());
    const lst_contract = formEntries.lst_contract.toString();
    const msg = {
      bond_rewards: {}
    };

    console.log(JSON.stringify(msg));
    try {
      const funds: any[] = [];
      const res = await client?.execute(userAddress, lst_contract, msg, "auto", "execute bond", funds);
      alert(res?.transactionHash);
      console.log(res?.transactionHash);
    } catch (err) {
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
