"use client";

import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
    Select,
    SelectItem
} from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";
import { MsgExecuteContract } from "cosmjs-types/cosmwasm/wasm/v1/tx";
import { toUtf8 } from "@cosmjs/encoding";
import { getTimeoutInNanoseconds7DaysFromNow } from "@/app/lib/utils";
import { encodeInstruction, encodeTokenOrderV2, tokenOrderV2WithSolverMetadata, U_FROM_UNION_SOLVER_METADATA_TESTNET } from "@/app/lib/ucs03";
import { Instruction } from "@unionlabs/sdk/Ucs03";
import { getSalt } from "@/app/lib/utils";
import { useState } from "react";


export const chains = [
    { key: "sepolia", label: "Sepolia" },
    { key: "holesky", label: "Holesky" },
];

interface KeyProps {
    stateKey: number;
    setStateKey: (key: number) => void;
}

export default function TransferU({ stateKey, setStateKey }: KeyProps) {
    const { userAddress, client, network } = useGlobalContext();
    const [isLoading, setIsLoading] = useState(false);

    const ucs03_contract = network?.escher?.ucs03;
    const receiver = "0x15Ee7c367F4232241028c36E720803100757c6e9";

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        setIsLoading(true);
        e.preventDefault();
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const amount = BigInt(formEntries.amount.toString());
        const recipient = formEntries.recipient.toString();
        const destination_chain = formEntries.destination_chain.toString();

        if (userAddress === undefined || userAddress === null) {
            alert("Please connect your wallet");
            return;
        }

        const quoteToken = network?.escher?.channel[destination_chain].nativeQuoteToken;
        if (!quoteToken || !network?.escher?.nativeBaseToken) {
            alert("no quote token or no native base token");
            return;
        }
        let tokenOrder =
            tokenOrderV2WithSolverMetadata(userAddress.toLowerCase(), recipient, network?.escher?.nativeBaseToken, amount, quoteToken as '0x${string}', amount, U_FROM_UNION_SOLVER_METADATA_TESTNET);

        let msg = {
            send: {
                channel_id: network?.escher?.channel[destination_chain]?.sourceChannelId,
                timeout_height: "0",
                timeout_timestamp: getTimeoutInNanoseconds7DaysFromNow().toString(),
                salt: getSalt(),
                instruction: encodeInstruction(Instruction.make({
                    opcode: 3,
                    version: 2,
                    operand: encodeTokenOrderV2(tokenOrder),
                })),
            },
        }

        const executeTransferMsg = {
            typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
            value: MsgExecuteContract.fromPartial({
                sender: userAddress,
                contract: ucs03_contract,
                msg: toUtf8(JSON.stringify(msg)),
                funds: [{
                    denom: network?.stakeCurrency.coinMinimalDenom,
                    amount: amount.toString()
                }]
            }),
        };

        try {
            const res = await client?.signAndBroadcast(userAddress, [executeTransferMsg], "auto", "transfer u");
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
                            name="recipient"
                            label="Recipient"
                            defaultValue={receiver}
                        />
                        <Select className="max-w-xs" label="Select destination chain" variant="flat" name="destination_chain">
                            {chains.map((chain) => (
                                <SelectItem key={chain.key}>{chain.label}</SelectItem>
                            ))}
                        </Select>
                        <div className="text-sm italic p-1">
                            Note: To send to sepolia, after send the packet need to run curl to relay (see README at client folder for CURL example)
                        </div>
                    </CardBody>
                    <CardFooter>
                        <Button type="submit" isLoading={isLoading}>Submit</Button>
                    </CardFooter>
                </Card>
            </form>
        </div >
    );
}