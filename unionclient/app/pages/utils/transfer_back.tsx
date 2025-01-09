import React, { useState } from "react";
import {
  Autocomplete,
  AutocompleteItem,
  Card,
  CardBody,
  Button,
  Input,
} from "@nextui-org/react";
import { defaultRegistryTypes } from "@cosmjs/stargate";
import { wasmTypes } from "@cosmjs/cosmwasm-stargate";
import { useGlobalContext } from "@/app/core/context";
import { osmosisProtoRegistry } from "@osmosis-labs/proto-codecs";

const allTypes = [
  ...defaultRegistryTypes,
  ...wasmTypes,
  ...osmosisProtoRegistry,
];
export const getMessage = (msg: { typeUrl: string; value: Object }) => {
  const { typeUrl, value } = msg;

  const encoder = allTypes.find((type) => type[0] === typeUrl);

  if (!encoder) throw new Error(`Message type ${typeUrl} not found`);

  const any = encoder[1].encode(value).finish();
  return {
    type_url: typeUrl,
    value: Buffer.from(any).toString("base64"),
  };
};

const TransferBack = () => {
  const { client, userAddress, icaControllerAddress, icaAddress } =
    useGlobalContext();
  const [isExecuting, setIsExecuting] = useState(false);

  const handleSubmit = async (e: any) => {
    setIsExecuting(true);
    try {
      e.preventDefault();
      const form = e.target;
      const formData = new FormData(form);
      const formEntries = Object.fromEntries(formData.entries());

      const ica_address = formEntries.ica_address.toString();
      const amount = formEntries.amount.toString();
      const denom = formEntries.denom.toString();
      const ica_controller_address = formEntries.ica_controller_address
        ? formEntries.ica_controller_address.toString()
        : undefined;
      const ica_transfer_channel_id = formEntries.ica_transfer_channel_id
        ? formEntries.ica_transfer_channel_id.toString()
        : undefined;

      const token = {
        amount,
        denom,
      };

      let transfer_from_i_c_a = {
        ica_address,
        token,
        ica_controller_address,
        ica_transfer_channel_id,
      };

      const msg = {
        transfer_from_i_c_a,
      };

      //const res = await client?.execute(userAddress, walletAddress, msg, "auto");

      //alert(res?.transactionHash);
      //console.log(JSON.stringify(res));
    } catch (e) {
      console.log(e);
      console.log("Failed to execute");
    }

    setIsExecuting(false);
  };

  return (
    <div className="font-sans text-center mx-auto">
      <Card>
        <form onSubmit={handleSubmit}>
          <CardBody className="gap-4">
            <Input
              name="ica_address"
              isRequired
              label="ICA Address"
              defaultValue={icaAddress ? icaAddress : ""}
            />
            <Input
              name="amount"
              label="Amount"
              className="max-w-xs"
              defaultValue="0"
            />
            <Input
              name="denom"
              isRequired
              label="Denom"
              className="max-w-xs"
              defaultValue="uosmo"
            />
            <Input
              name="ica_controller_address"
              label="ICA Controller Address"
              className="max-w-xs"
              defaultValue={icaControllerAddress ? icaControllerAddress : ""}
            />
            <Input
              name="ica_transfer_channel_id"
              label="ICA Source Channel"
              className="max-w-xs"
              defaultValue="channel-7779"
            />

            <Button type="submit" disabled={isExecuting}>
              {isExecuting ? "Executing ..." : "Execute"}
            </Button>
          </CardBody>
        </form>
      </Card>
    </div>
  );
};

export default TransferBack;
