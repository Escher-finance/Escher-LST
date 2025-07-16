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
import { BaseNetworks } from "@/config/networks.config";

interface KeyProps {
    stateKey: number;
    setStateKey: (key: number) => void;
}

export default function ZkgmUnbond({ stateKey, setStateKey }: KeyProps) {
    const [isLoading, setIsLoading] = useState(false);

    const { userAddress, client, network } = useGlobalContext();
    const ucs03_contract = network?.escher.ucs03;
    const channel_id = network?.escher.channel["babylon"]?.sourceChannelId;
    const baby_lst_contract = network?.escher.lst;
    const ebaby_denom = network?.escher.ebabyDenom;

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
                recipient_ibc_channel_id: network?.escher.channel["babylon"]?.destinationIbcChannelId,
            }
        };

        console.log("Payload: ", JSON.stringify(payload));

        let testnet = network?.chainName?.toLowerCase().includes("testnet");

        const cosmosIntent: TransferAndCallIntent = {
            sender: userAddress,
            receiver: baby_lst_contract,
            baseToken: formEntries.denom.toString(),
            baseAmount: BigInt(amount),
            baseTokenSymbol: "eBABY",
            baseTokenName: "ebbn",
            quoteToken: testnet ? toHex(BaseNetworks["babylon-testnet"].escher.ebabyDenom) : toHex(BaseNetworks["babylon-mainnet"].escher.ebabyDenom),
            quoteAmount: BigInt(amount),
            baseTokenPath: BigInt(network?.escher.channel["babylon"]?.sourceChannelId),
            payload
        } as const

        const batch_instruction = transferAndCallInstruction(cosmosIntent);

        console.log(JSON.stringify(batch_instruction));

        const timeout_timestamp = getTimeoutInNanoseconds24HoursFromNow().toString();

        let msg = {
            send: {
                channel_id,
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
                denom: network?.escher.ebabyDenom
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
            console.log(err);
            setIsLoading(false);
        }
    };

    return (
        <div className="w-full flex flex-col gap-4">
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