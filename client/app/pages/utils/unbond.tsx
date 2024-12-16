"use client";

import {
  Card,
  CardBody,
  CardFooter,
  Button,
  Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";
import { getExecuteContractMessage } from "@/utils/msg";

export default function Unbond() {
  const { userAddress, client, network } = useGlobalContext();


  const handleSubmit = async (e: any) => {
    // Prevent the browser from reloading the page
    e.preventDefault();
    const form = e.target;
    const formData = new FormData(form);
    const formEntries = Object.fromEntries(formData.entries());
    const amount = formEntries.amount.toString();


    try {
      if (!userAddress) {
        alert("no user wallet");
        return;
      }

      const transferCW20TokenMsg = {
        transfer: {
          recipient: network?.contracts.lst,
          amount
        },
      };
      const funds = [
        {
          denom: "lqstake",
          amount
        }
      ];
      const executeTransferCW20Msg = getExecuteContractMessage(userAddress, network?.contracts.cw20, transferCW20TokenMsg, []);

      const unbondingMsg = {
        unbond: {
          staker: userAddress,
          amount
        }
      };
      console.log(JSON.stringify(unbondingMsg));
      const executeUnbondingMsg = getExecuteContractMessage(userAddress, network?.contracts.lst, unbondingMsg, []);


      let msgs = [executeTransferCW20Msg, executeUnbondingMsg];
      const res = await client.signAndBroadcast(userAddress, msgs, "auto", "");
      alert(res.transactionHash);
      console.log(res.transactionHash);

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
            <Button type="submit">Unbond</Button>
          </CardFooter>
        </Card>
      </form>
    </div>
  );
}

