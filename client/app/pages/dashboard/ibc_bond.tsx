"use client";

import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";
import { createSendIBCMsg, getTimeoutInNanoseconds24HoursFromNow } from "@/app/lib/ibc";
import { getSalt } from "@/app/lib/salt";
import { useState } from "react";
import { toHex } from "viem";


const baby_denom = "ibc/D538E142FC525F8CE0937169ED4645AE6E5BFC37F9C2C5CB178603F5DF1FEDF3"; // testnet denom
// const baby_denom = "ibc/EC3A4ACBA1CFBEE698472D3563B70985AEA5A7144C319B61B3EBDFB57B5E1535"; // Replace with the actual baby denom on Osmosis

const sourceChannel = "channel-10366"; // Testnet ibc channel id from osmosis to babylon testnet
// const sourceChannel = "channel-101635"; // Replace with the actual source channel

const default_recipient = "osmo1vnglhewf3w66cquy6hr7urjv3589srhelhn6df"; // Replace with the actual recipient address

interface KeyProps {
    stateKey: number;
    setStateKey: (key: number) => void;
}

export default function IbcBond({ stateKey, setStateKey }: KeyProps) {
    const { userAddress, client, network } = useGlobalContext();
    const [isLoading, setIsLoading] = useState(false);

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const recipient_channel_id = formEntries.recipient_channel_id.toString();
        const recipient = formEntries.recipient.toString();

        const amount = BigInt(formEntries.amount.toString());

        if (userAddress === undefined || userAddress === null) {
            alert("Please connect your wallet");
            return;
        }

        const rate = 1.509253;

        let payload = {
            dest_callback: {
                address: network?.contracts.lst
            },
            salt: getSalt(),
            amount: amount.toString(),
            recipient: recipient_channel_id == "0" ? recipient : toHex(recipient),
            recipient_channel_id: recipient_channel_id == "0" ? null : recipient_channel_id,
            expected: (Number(amount) / rate).toFixed().toString(),
        };

        console.log("Payload:", JSON.stringify(payload));


        const msg = createSendIBCMsg({
            sender: userAddress,
            denom: baby_denom,
            amount: (Number(amount) + 10000).toString(),
            sourceChannel,
            receiver: network?.contracts.lst,
            timeoutTimestamp: getTimeoutInNanoseconds24HoursFromNow(),
            memo: JSON.stringify(payload),
        });

        try {
            const res = await client?.signAndBroadcast(userAddress, [msg], "auto", "stake to babylon from osmosis");
            alert(res?.transactionHash);

            let newKey = stateKey + 1;
            setStateKey(newKey);
            setIsLoading(false);
        } catch (err) {
            setIsLoading(false);
            console.log(err);
        }
    }

    return (
        <div className="w-full flex flex-col gap-4">
            <div className="p-3">
                IBC BOND
            </div>
            <form onSubmit={handleSubmit} className="w-full flex">
                <Card className="w-full flex">
                    <CardBody className="gap-4">
                        <Input
                            isRequired
                            name="amount"
                            label="Amount"
                            defaultValue="10000"
                        />
                        <Input
                            isRequired
                            name="denom"
                            label="Denom (baby denom on osmosis)"
                            defaultValue={baby_denom}
                        />
                        <Input
                            isRequired
                            name="recipient"
                            label="Recipient Address"
                            defaultValue={default_recipient}
                        />
                        <Input
                            isRequired
                            name="recipient_channel_id"
                            label="Recipient Channel ID (set to 0 if recipient is at babylon)"
                            defaultValue="5"
                        />
                    </CardBody>
                    <CardFooter>
                        <Button type="submit" isLoading={isLoading}>Submit</Button>
                    </CardFooter>
                </Card>
            </form>
        </div>
    );
}
