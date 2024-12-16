"use client";

import {
  Card,
  CardBody,
  CardFooter,
  Button,
  Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";

export default function Mint() {
  const { userAddress, client, network } = useGlobalContext();

  const handleSubmit = async (e: any) => {
    // Prevent the browser from reloading the page
    e.preventDefault();
    const form = e.target;
    const formData = new FormData(form);
    const formEntries = Object.fromEntries(formData.entries());
    const lst_contract = formEntries.lst_contract.toString();
    const msg = {
        mint : {}
    };

    console.log(JSON.stringify(msg));
    try {
      const res = await client.execute(userAddress, lst_contract, msg, "auto", "execute bond", []);
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
              name="lst_contract"
              label="LiquidStaking Contract"
              defaultValue="union1gsf8pggh5tnr0n6g6qkq4qs4exsru0ke2re7jh95chsvrjwj0tkq6p8a70"
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
