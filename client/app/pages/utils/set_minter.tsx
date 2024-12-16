"use client";

import {
  Card,
  CardBody,
  CardFooter,
  Button,
  Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";

export default function SetMinter() {
  const { userAddress, client, network } = useGlobalContext();

  const handleSubmit = async (e: any) => {
    // Prevent the browser from reloading the page
    e.preventDefault();
    const form = e.target;
    const formData = new FormData(form);
    const formEntries = Object.fromEntries(formData.entries());
    const new_minter = formEntries.new_minter.toString();
    const cw20_contract = formEntries.cw20_contract.toString();
    const msg = {
      update_minter: {
        new_minter
      }
    };

    console.log(JSON.stringify(msg));
    try {
      const res = await client.execute(userAddress, cw20_contract, msg, "auto");
      alert(res.transactionHash);

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
              name="cw20_contract"
              label="CW20 Contract"
              defaultValue={network?.contracts.cw20}
            />
            <Input
              isRequired
              name="new_minter"
              label="New Minter"
              defaultValue={network?.contracts.lst}
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