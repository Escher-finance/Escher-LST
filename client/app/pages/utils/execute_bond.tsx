"use client";

import {
  Card,
  CardBody,
  CardFooter,
  Button,
  Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";

export default function ExecuteBond() {
  const { userAddress, client, network } = useGlobalContext();

  const handleSubmit = async (e: any) => {
    e.preventDefault();
    const form = e.target;
    const formData = new FormData(form);
    const formEntries = Object.fromEntries(formData.entries());
    const amount = formEntries.amount.toString();
    const msg = {
      bond: {
        staker: userAddress
      }
    };

    console.log(JSON.stringify(msg));
    try {
      const funds = [{
        amount,
        denom: "stake"
      }];
      const res = await client.execute(userAddress, network?.contracts.lst, msg, "auto", "execute bond", funds);
      alert(res.transactionHash);

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
              name="amount"
              label="Amount"
              defaultValue="0"
            />
          </CardBody>
          <CardFooter>
            <Button type="submit">Bond</Button>
          </CardFooter>
        </Card>
      </form>
    </div>
  );
}
