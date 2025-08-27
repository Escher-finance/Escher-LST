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
import { getSalt, transferInstruction, TransferIntent } from "../utils/ucs03";
import { MsgExecuteContract } from "cosmjs-types/cosmwasm/wasm/v1/tx";
import { toUtf8 } from "@cosmjs/encoding";
import { getTimeoutInNanoseconds24HoursFromNow } from "@/app/lib/ibc";
import { Instruction } from "@unionlabs/sdk/ucs03";
import { BaseNetworks } from "@/config/networks.config";


// Get quote token of baby in osmosis 
// osmosisd query wasm contract-state smart osmo1336jj8ertl8h7rdvnz4dh5rqahd09cy0x43guhsxx6xyrztx292qs2uecc '{"predict_wrapped_token":{"channel_id":3, "path":"0", "token":"0x7562626e"}}'
// data:
// wrapped_token: 0x666163746f72792f6f736d6f3133756c63367071686d3630716e78353873733773336366743863716679636578713375793364643276306c3871736e6b766b34736a3232736e362f46374266536e58746d6652613343475541473841507055576b42794476686445706e464874694b59394542

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


export default function TransferEbabyToOsmosis() {
    const { userAddress, client, network } = useGlobalContext();

    const base_denom = network?.escher?.ebabyDenom;
    const ucs03_contract = network?.escher?.ucs03;
    const channel_id = network?.escher?.channel["osmosis"]?.sourceChannelId;
    const receiver = "osmo1vnglhewf3w66cquy6hr7urjv3589srhelhn6df"
    const zkgm_token_minter = network?.escher?.tokenMinter;


    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const amount = BigInt(formEntries.amount.toString());

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

        console.log("msg", JSON.stringify(msg));

        let funds = [{
            denom: "ubbn",
            amount: testnet ? "10000" : "100"
        }]

        const executeTransferMsg = {
            typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
            value: MsgExecuteContract.fromPartial({
                sender: userAddress,
                contract: ucs03_contract,
                msg: toUtf8(JSON.stringify(msg)),
                funds
            }),
        };

        const executeAllowanceMsg = getExecuteAllowanceMsg(formEntries.denom.toString(), userAddress, zkgm_token_minter, amount.toString());


        console.log(JSON.stringify(msg));
        try {
            const res = await client?.signAndBroadcast(userAddress, [executeAllowanceMsg, executeTransferMsg], "auto", "transfer ebaby from babylon to osmosis");
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
                            defaultValue="1000"
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
                            label="Receiver (osmosis address)"
                            defaultValue={receiver}
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

