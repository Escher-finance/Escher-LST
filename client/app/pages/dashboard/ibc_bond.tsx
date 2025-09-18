"use client";

import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
} from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";
import { createSendIBCMsg, getTimeoutInNanoseconds24HoursFromNow } from "@/app/lib/ibc";
import { toHex } from "viem";
import { getSalt } from "@/app/lib/utils";
import { BaseNetworks } from "@/config/networks.config";
import { useState } from "react";


const transfer_fee = BigInt(0);
interface KeyProps {
    stateKey: number;
    setStateKey: (key: number) => void;
}

export default function IbcBond({ stateKey, setStateKey }: KeyProps) {
    const [isLoading, setIsLoading] = useState(false);
    const { userAddress, client, network } = useGlobalContext();

    const baby_denom = network?.escher?.babyDenom;


    const testnet = network?.chainName.includes("testnet") ? true : false;
    const baby_lst_contract = testnet ? BaseNetworks["babylon-testnet"].escher.lst : BaseNetworks["babylon-mainnet"].escher.lst;
    const sourceChannel = network?.escher?.channel["babylon"]?.sourceIbcChannelId;

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const amount = BigInt(formEntries.amount.toString());
        const recipient = formEntries.recipient.toString();
        const recipient_channel_id = formEntries.recipient_channel_id.toString();

        if (userAddress === undefined || userAddress === null) {
            alert("Please connect your wallet");
            return;
        }

        const exchange_rate = Number(formEntries.exchange_rate);
        console.log("Exchange Rate:", exchange_rate);
        const expected = Math.floor(Number(amount) / exchange_rate).toString();

        let payload = {
            dest_callback: {
                address: baby_lst_contract
            },
            salt: getSalt(),
            amount: amount.toString(),
            recipient: recipient_channel_id == "" ? recipient : toHex(recipient),
            recipient_channel_id: recipient_channel_id == "" ? null : Number(recipient_channel_id),
            expected,
            transfer_fee: transfer_fee.toString(),
        };

        console.log("payload", JSON.stringify(payload));

        const msg = createSendIBCMsg({
            sender: userAddress,
            denom: baby_denom,
            amount: recipient_channel_id != "" ? (amount + transfer_fee).toString() : amount.toString(),
            sourceChannel,
            receiver: baby_lst_contract,
            timeoutTimestamp: getTimeoutInNanoseconds24HoursFromNow(),
            memo: JSON.stringify(payload),
        });

        console.log(JSON.stringify(msg));
        try {
            const res = await client?.signAndBroadcast(userAddress, [msg], "auto", "stake to babylon from osmosis");
            alert(res?.transactionHash);

            let newKey = stateKey + 1;
            setStateKey(newKey);
            setIsLoading(false);
        } catch (err) {
            console.log(err);
            setIsLoading(false);
        }
    };

    return (

        <div className="w-full flex flex-col">
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
                            name="Contract"
                            label="LST Contract"
                            defaultValue={baby_lst_contract}
                        />
                        <Input
                            isRequired
                            name="recipient"
                            label="Recipient Address"
                            defaultValue="osmo1vnglhewf3w66cquy6hr7urjv3589srhelhn6df"
                        />
                        <Input
                            isRequired
                            name="recipient_channel_id"
                            label="Recipient UCS03 Channel ID"
                            defaultValue="4"
                        />
                        <Input
                            isRequired
                            name="exchange_rate"
                            label="Exchange rate"
                            defaultValue="1.056"
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


//https://btc.union.build/explorer/packets/0xce8c32b71b5a7608b6b1afdea4fbb53c66cb026ed68916891d557277adbcfd4c

// - amount: "81427"
//   denom: factory/osmo13ulc6pqhm60qnx58ss7s3cft8cqfycexq3uy3dd2v0l8qsnkvk4sj22sn6/5dDrk51st6AKJwxbyFwe8wydD417XHRDAAx9JSJN7c9a
// - amount: "10000"
//   denom: factory/osmo13ulc6pqhm60qnx58ss7s3cft8cqfycexq3uy3dd2v0l8qsnkvk4sj22sn6/F7BfSnXtmfRa3CGUAG8APpUWkByDvhdEpnFHtiKY9EB