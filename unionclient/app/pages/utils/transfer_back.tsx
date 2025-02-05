import React, { useState } from "react";
import {
  Card,
  CardBody,
  Button,
  Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";
import { http } from "viem"
import { holesky } from "viem/chains";
import { DirectSecp256k1Wallet } from "@cosmjs/proto-signing";
import { bech32, hex, bytes } from "@scure/base"

export function hexToBytes(hexString: string): Uint8Array {
  return bytes("hex", hexString.indexOf("0x") === 0 ? hexString.slice(2) : hexString)
}

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

      const amount = formEntries.amount.toString();
      const denom = formEntries.denom.toString();
      const PRIVATE_KEY = "5101e25177a126b6c60291585e1f6f04e9d6da74d6cc7b4383cbb923f84b63d4";
      const cosmosAccount = await DirectSecp256k1Wallet.fromKey(
        Uint8Array.from(hexToBytes(PRIVATE_KEY)),
        "union"
      );

      console.log("cosmosAccount", JSON.stringify(cosmosAccount));
      // const unionclient = createUnionClient({
      //   account: cosmosAccount,
      //   chainId: "union-testnet-9",
      //   gasPrice: { amount: "0.0025", denom: "muno" },
      //   transport: http("https://rpc.testnet-9.union.build")
      // })

      // const transferPayload = {
      //   amount: BigInt(amount),
      //   denomAddress: denom,
      //   destinationChainId: `${holesky.id}`,
      //   receiver: "0x8478B37E983F520dBCB5d7D3aAD8276B82631aBd"
      // } satisfies TransferAssetsParameters<"union-testnet-8">


      // const transfer = await unionclient.transferAsset(transferPayload)
      // if (transfer.isErr()) {
      //   console.log(transfer.error)
      // }

      // console.log(JSON.stringify(transfer));
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
