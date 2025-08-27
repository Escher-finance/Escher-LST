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
import { createSendIBCMsg, getTimeoutInNanoseconds24HoursFromNow } from "@/app/lib/ibc";

const getExecuteAllowanceMsg = (contract: string, sender: string, spender: string, amount: string) => {

    let allowanceMsg = {
        increase_allowance: {
            spender,
            amount,
        }
    }
    console.log(JSON.stringify(allowanceMsg));
    const executeAllowanceMsg = {
        typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
        value: MsgExecuteContract.fromPartial({
            sender,
            contract,
            msg: toUtf8(JSON.stringify(allowanceMsg)),
            funds: []
        }),
    };

    return executeAllowanceMsg;
}


export default function DualTransferToOsmosis() {
    const { userAddress, client, network } = useGlobalContext();

    const base_denom = network?.escher.ebabyDenom;
    const baby_denom = network?.escher.babyDenom;

    const ucs03_contract = network?.escher?.ucs03;
    const channel_id = network?.escher?.channel["osmosis"]?.sourceChannelId;

    const default_receiver = "osmo1vnglhewf3w66cquy6hr7urjv3589srhelhn6df"; // Default receiver address for Babylon

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const receiver = formEntries.receiver.toString() || default_receiver;
        const amount = BigInt(formEntries.amount.toString());
        const zkgm_token_minter = network?.escher?.tokenMinter;


        if (userAddress === undefined || userAddress === null) {
            alert("Please connect your wallet");
            return;
        }

        let testnet = network?.chainName?.toLowerCase().includes("testnet");

        const cosmosIntent: TransferIntent = {
            sender: userAddress,
            receiver: formEntries.receiver.toString(),
            baseToken: formEntries.denom.toString(),
            baseAmount: BigInt(amount),
            baseTokenSymbol: "eBABY",
            baseTokenName: "ebbn",
            quoteToken: testnet ? toHex(BaseNetworks["osmosis-testnet"].escher.ebabyDenom) : toHex(BaseNetworks["osmosis-mainnet"].escher.ebabyDenom),
            quoteAmount: BigInt(amount),
            baseTokenPath: BigInt(0),
            baseTokenDecimals: 6
        } as const

        console.log("cosmosIntent", JSON.stringify(cosmosIntent));
        const batch_instruction = transferInstruction(cosmosIntent);

        console.log("batch_instruction", JSON.stringify(batch_instruction));

        const timeout_timestamp = getTimeoutInNanoseconds24HoursFromNow().toString();

        let ucs03_msg = {
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


        console.log(userAddress);

        const executeAllowanceMsg = getExecuteAllowanceMsg(formEntries.denom.toString(), userAddress, zkgm_token_minter, amount.toString());



        const executeUcs03Msg = {
            typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
            value: MsgExecuteContract.fromPartial({
                sender: userAddress,
                contract: ucs03_contract,
                msg: toUtf8(JSON.stringify(ucs03_msg)),
                funds: []
            }),
        };
        console.log(JSON.stringify(executeUcs03Msg));

        const ibcTransferMsg = createSendIBCMsg({
            sender: userAddress,
            denom: baby_denom,
            amount: amount.toString(),
            sourceChannel: network?.escher.channel["osmosis"]?.sourceIbcChannelId,
            receiver,
            timeoutTimestamp: getTimeoutInNanoseconds24HoursFromNow(),
            memo: "Transfer baby from babylon to osmosis",
        });


        console.log(JSON.stringify(ibcTransferMsg));
        try {
            const res = await client?.signAndBroadcast(userAddress, [executeAllowanceMsg, executeUcs03Msg, ibcTransferMsg], "auto", "transfer baby and ebaby from babylon to osmosis");
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
                            label="eBaby Denom"
                            defaultValue={base_denom}
                        />
                        <Input
                            isRequired
                            name="receiver"
                            label="Receiver (osmosis address)"
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

