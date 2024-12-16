"use client";

import {
  Card,
  CardBody,
  CardFooter,
  Button,
  Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";

export default function InstantiateICA() {
  const { userAddress, client, network } = useGlobalContext();

  const handleSubmit = async (e: any) => {
    // Prevent the browser from reloading the page
    e.preventDefault();
    const form = e.target;
    const formData = new FormData(form);
    const msg = {
      owner: userAddress,
      channel_open_init_options: {
        connection_id: network?.connectionID,
        counterparty_connection_id: network?.counterPartyConnectionID,
        counterparty_port_id: network?.counterpartyPortID,
        channel_ordering: "ORDER_ORDERED",
      }
    };

    console.log(JSON.stringify(msg));
    try {
      const instantiateOptions = {
        memo: "Instantiating a new contract",
        funds: [],
        admin: userAddress,
      }

      const instantiateResult = await client.instantiate(
        userAddress,
        network?.contracts.icaController?.codeId,
        msg,
        "nomos-ica-controller",
        "auto",
        instantiateOptions
      );
      console.log(instantiateResult.contractAddress);
      alert(instantiateResult.contractAddress);

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
              label="Factory CodeID"
              className="max-w-xs"
              defaultValue={network?.contracts?.icaController?.codeId.toString()}
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
