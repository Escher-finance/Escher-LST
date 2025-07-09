"use client";

import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";
import { encodeAbiParameters, Hex, toHex } from "viem";
import { instructionAbi } from "@unionlabs/sdk/evm/abi";
import { getSalt, transferAndCallInstruction, TransferAndCallIntent, getTimeoutInNanoseconds24HoursFromNow } from "../utils/ucs03";
import { MsgExecuteContract } from "cosmjs-types/cosmwasm/wasm/v1/tx";
import { toUtf8 } from "@cosmjs/encoding";
import { Instruction } from "@unionlabs/sdk/ucs03";
import { useState } from "react";

const baby_lst_contract = "bbn1ug4tume0pw6d4u7r6rhae6cp3udyrv7cr0angx8qegw7ur25sdxq4krcss";
const ucs03_contract = "osmo1336jj8ertl8h7rdvnz4dh5rqahd09cy0x43guhsxx6xyrztx292qs2uecc";

const ebaby_denom = "factory/osmo13ulc6pqhm60qnx58ss7s3cft8cqfycexq3uy3dd2v0l8qsnkvk4sj22sn6/5dDrk51st6AKJwxbyFwe8wydD417XHRDAAx9JSJN7c9a";

interface KeyProps {
    stateKey: number;
    setStateKey: (key: number) => void;
}

export default function ZkgmUnbond({ stateKey, setStateKey }: KeyProps) {


    const [isLoading, setIsLoading] = useState(false);

    const { userAddress, client } = useGlobalContext();

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const amount = BigInt(formEntries.amount.toString());
        const recipient = formEntries.recipient.toString();

        if (userAddress === undefined || userAddress === null) {
            alert("Please connect your wallet");
            return;
        }

        let payload = {
            unbond: {
                amount: amount.toString(),
                recipient,
                recipient_ibc_channel_id: "channel-21"
            }
        };


        const cosmosIntent: TransferAndCallIntent = {
            sender: userAddress,
            receiver: baby_lst_contract,
            baseToken: formEntries.denom.toString(),
            baseAmount: BigInt(amount),
            baseTokenSymbol: "eBABY",
            baseTokenName: "ebbn",
            quoteToken: toHex("bbn1cnx34p82zngq0uuaendsne0x4s5gsm7gpwk2es8zk8rz8tnj938qqyq8f9"),
            quoteAmount: BigInt(amount),
            baseTokenPath: BigInt(3),
            payload
        } as const

        const batch_instruction = transferAndCallInstruction(cosmosIntent);

        console.log(JSON.stringify(batch_instruction));

        const timeout_timestamp = getTimeoutInNanoseconds24HoursFromNow().toString();

        let msg = {
            send: {
                channel_id: 3,
                timeout_height: "0",
                timeout_timestamp,
                salt: getSalt(),
                instruction: encodeAbiParameters(instructionAbi, [
                    0,
                    2,
                    Instruction.encodeAbi(batch_instruction)
                ])
            },
        }

        let funds = [
            {
                amount: amount.toString(),
                denom: "factory/osmo13ulc6pqhm60qnx58ss7s3cft8cqfycexq3uy3dd2v0l8qsnkvk4sj22sn6/5dDrk51st6AKJwxbyFwe8wydD417XHRDAAx9JSJN7c9a"
            }
        ];

        const executeBondMsg = {
            typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
            value: MsgExecuteContract.fromPartial({
                sender: userAddress,
                contract: ucs03_contract,
                msg: toUtf8(JSON.stringify(msg)),
                funds
            }),
        };


        console.log(JSON.stringify(msg));
        try {
            const res = await client?.signAndBroadcast(userAddress, [executeBondMsg], "auto", "unbond from osmosis to babylon");
            alert(res?.transactionHash);
            let newKey = stateKey + 1;
            setStateKey(newKey);
            setIsLoading(false);
        } catch (err) {
            setIsLoading(false);
            console.log(err);
        }
    };

    return (
        <div className="w-full flex flex-col gap-4">
            <div className="p-1 text-xl">
                ZKGM UNBOND
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
                            label="Denom"
                            defaultValue={ebaby_denom}
                        />
                        <Input
                            isRequired
                            name="recipient"
                            label="Recipient"
                            defaultValue={userAddress}
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