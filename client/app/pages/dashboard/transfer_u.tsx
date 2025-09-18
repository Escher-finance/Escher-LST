"use client";

import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
    Select,
    SelectItem
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";
import { encodeAbiParameters, Hex, toHex } from "viem";
import { MsgExecuteContract } from "cosmjs-types/cosmwasm/wasm/v1/tx";
import { toUtf8 } from "@cosmjs/encoding";
import { getTimeoutInNanoseconds24HoursFromNow } from "@/app/lib/utils";
import { encodeTokenOrderV2, tokenOrderV2 } from "@/app/lib/ucs03";
import { InstructionAbi } from "@unionlabs/sdk/Ucs03";
import { getSalt } from "@/app/lib/utils";
import { useState } from "react";


// Get quote token of baby in osmosis 
// osmosisd query wasm contract-state smart osmo1336jj8ertl8h7rdvnz4dh5rqahd09cy0x43guhsxx6xyrztx292qs2uecc '{"predict_wrapped_token":{"channel_id":3, "path":"0", "token":"0x7562626e"}}'
// data:
// wrapped_token: 0x666163746f72792f6f736d6f3133756c63367071686d3630716e78353873733773336366743863716679636578713375793364643276306c3871736e6b766b34736a3232736e362f46374266536e58746d6652613343475541473841507055576b42794476686445706e464874694b59394542


export const chains = [
    { key: "sepolia", label: "Sepolia" },
    { key: "holesky", label: "Holesky" },
];


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

interface KeyProps {
    stateKey: number;
    setStateKey: (key: number) => void;
}

export default function TransferU({ stateKey, setStateKey }: KeyProps) {
    const { userAddress, client, network } = useGlobalContext();
    const [isLoading, setIsLoading] = useState(false);

    const base_denom = network?.escher?.ebabyDenom;
    const ucs03_contract = network?.escher?.ucs03;
    const receiver = "0x15Ee7c367F4232241028c36E720803100757c6e9"
    const zkgm_token_minter = network?.escher?.tokenMinter;


    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        setIsLoading(true);
        e.preventDefault();
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const amount = BigInt(formEntries.amount.toString());
        const recipient = formEntries.receiver.toString();
        const destination_chain = formEntries.destination_chain.toString();

        const channel_id = network?.escher?.channel[destination_chain]?.sourceChannelId;

        if (userAddress === undefined || userAddress === null) {
            alert("Please connect your wallet");
            return;
        }
        const executeAllowanceMsg = getExecuteAllowanceMsg(formEntries.denom.toString(), userAddress, zkgm_token_minter, amount.toString());

        let testnet = network?.chainName?.toLowerCase().includes("testnet");

        const quoteToken = network?.escher?.channel[destination_chain].nativeQuoteToken;
        if (!quoteToken || !network?.escher?.nativeBaseToken) {
            alert("no quote token or no native base token");
            return;
        }
        let tokenOrder =
            tokenOrderV2(userAddress.toLowerCase(), recipient, network?.escher?.nativeBaseToken, amount, quoteToken as '0x${string}', amount);

        const timeout_timestamp = getTimeoutInNanoseconds24HoursFromNow().toString();

        let msg = {
            send: {
                channel_id,
                timeout_height: "0",
                timeout_timestamp,
                salt: getSalt(),
                instruction: encodeAbiParameters(InstructionAbi(), [
                    tokenOrder.opcode,
                    tokenOrder.version,
                    encodeTokenOrderV2(tokenOrder)
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


        console.log(JSON.stringify(msg));
        try {
            const res = await client?.signAndBroadcast(userAddress, [executeAllowanceMsg, executeTransferMsg], "auto", "transfer ebaby from babylon to osmosis");
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
                            name="recipient"
                            label="Recipient"
                            defaultValue={receiver}
                        />
                        <Select className="max-w-xs" label="Select destination chain" variant="flat">
                            {chains.map((chain) => (
                                <SelectItem key={chain.key}>{chain.label}</SelectItem>
                            ))}
                        </Select>
                    </CardBody>
                    <CardFooter>
                        <Button type="submit" isLoading={isLoading}>Submit</Button>
                    </CardFooter>
                </Card>
            </form>
        </div >
    );
}


// https://btc.union.build/explorer/packets/0xce8c32b71b5a7608b6b1afdea4fbb53c66cb026ed68916891d557277adbcfd4c

// https://btc.union.build/explorer/transfers/0x0c3399bc587f9a6934d19d82fc1fef512c0f035c72858b83534b6919feaf36a0

