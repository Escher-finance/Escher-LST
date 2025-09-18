"use client";

import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";
import { encodeAbiParameters, toHex } from "viem";
import { instructionAbi } from "@unionlabs/sdk/evm/abi";
import { getSalt, transferInstruction, TransferIntent } from "../utils/ucs03";
import { MsgExecuteContract } from "cosmjs-types/cosmwasm/wasm/v1/tx";
import { toUtf8 } from "@cosmjs/encoding";
import { Instruction } from "@unionlabs/sdk/ucs03";
import { BaseNetworks } from "@/config/networks.config";
import { getTimeoutInNanoseconds24HoursFromNow } from "@/app/lib/ibc";

export default function TransferEbabyFromOsmosis() {
    const { userAddress, client, network } = useGlobalContext();

    const base_denom = network?.escher.ebabyDenom;

    const ucs03_contract = network?.escher.ucs03;
    const channel_id = network?.escher.channel["babylon"]?.sourceChannelId;
    const default_receiver = "bbn1vnglhewf3w66cquy6hr7urjv3589srheqj3myz"; // Default receiver address for Babylon

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const receiver = formEntries.receiver.toString() || default_receiver;
        const amount = BigInt(formEntries.amount.toString());

        if (userAddress === undefined || userAddress === null) {
            alert("Please connect your wallet");
            return;
        }

        let testnet = network?.chainName?.toLowerCase().includes("testnet");

        const cosmosIntent: TransferIntent = {
            sender: userAddress,
            receiver,
            baseToken: formEntries.denom.toString(),
            baseAmount: BigInt(amount),
            baseTokenSymbol: "eBABY",
            baseTokenName: "ebbn",
            quoteToken: testnet ? toHex(BaseNetworks["babylon-testnet"].escher.ebabyDenom) : toHex(BaseNetworks["babylon-mainnet"].escher.ebabyDenom),
            quoteAmount: BigInt(amount),
            baseTokenPath: BigInt(network?.escher.channel["babylon"]?.sourceChannelId),
            baseTokenDecimals: 6
        } as const

        console.log("cosmosIntent", JSON.stringify(cosmosIntent));
        const batch_instruction = transferInstruction(cosmosIntent);

        console.log("batch_instruction", JSON.stringify(batch_instruction));

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



        console.log(JSON.stringify(funds));
        console.log(userAddress);

        const transferMsg = {
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
            const res = await client?.signAndBroadcast(userAddress, [transferMsg], "auto", "transfer ebaby from osmosis to babylon");
            alert(res?.transactionHash);

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
                            defaultValue="100"
                        />
                        <Input
                            isRequired
                            name="denom"
                            label="Denom"
                            defaultValue={base_denom}
                        />
                        <Input
                            isRequired
                            name="receiver"
                            label="Receiver (babylon address)"
                            defaultValue={default_receiver}
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


// https://btc.union.build/explorer/packets/0xce8c32b71b5a7608b6b1afdea4fbb53c66cb026ed68916891d557277adbcfd4c

// https://btc.union.build/explorer/transfers/0x0c3399bc587f9a6934d19d82fc1fef512c0f035c72858b83534b6919feaf36a0

