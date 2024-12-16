"use client";

import {
  Card,
  CardBody,
  CardFooter,
  Button,
  Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";
import {PressEvent} from "@react-types/shared";

export default function Migrate() {
  const { userAddress, client } = useGlobalContext();

  const handleSubmit = async (e: any) => {
    // Prevent the browser from reloading the page
    e.preventDefault();
    const form = e.target;
    const formData = new FormData(form);
    const formEntries = Object.fromEntries(formData.entries());
    const code_id = Number(formEntries.code_id);
    const lst_contract = formEntries.lst_contract.toString();
    const msg = {
    };

    console.log(JSON.stringify(msg));
    try {
      const res = await client.migrate(userAddress, lst_contract, code_id, msg, "auto");
      alert(res.transactionHash);

    } catch (err) {
      console.log(err);
    }
  };

  const reset = async (e: PressEvent) => {

    let lst_contract = "union1x70fmdv965fj6hm4lmyudxyphl6j9vweukmc3fxja3mamgqrup6qf9mv3x";
    try {
      let msg = {
        reset: {}
      };
      const res = await client.execute(userAddress, lst_contract, msg, "auto");
      alert(res.transactionHash);

    } catch (err) {
      console.log(err);
    }

  }

  return (
    <div className="w-full flex-col flex">
      <form onSubmit={handleSubmit} className="w-full flex">
        <Card className="grow">
          <CardBody className="w-full gap-4">
            <Input
              isRequired
              name="lst_contract"
              label="LiquidStaking Contract"
              defaultValue="union1x70fmdv965fj6hm4lmyudxyphl6j9vweukmc3fxja3mamgqrup6qf9mv3x"
            />
          </CardBody>
          <CardBody className="w-full gap-4">
            <Input
              isRequired
              name="code_id"
              label="Code ID"
              defaultValue="313"
            />
          </CardBody>
          <CardFooter>
            <Button type="submit">Submit</Button>
          </CardFooter>
        </Card>
      </form>
      <Card>
        <CardBody className="w-full gap-4">
          <div>
            <Button onPress={reset}>Reset</Button>
          </div>
        </CardBody>
      </Card>
    </div>
  );
}
