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
import { encodeInstruction, encodeTokenOrderV2, EU_FROM_UNION_SOLVER_METADATA_TESTNET, tokenOrderV2WithSolverMetadata } from "@/app/lib/ucs03";
import { Instruction } from "@unionlabs/sdk/Ucs03";
import { getSalt } from "@/app/lib/utils";
import { useState } from "react";


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

export default function TransfereU({ stateKey, setStateKey }: KeyProps) {
    const { userAddress, client, network } = useGlobalContext();
    const [isLoading, setIsLoading] = useState(false);

    const ucs03_contract = network?.escher?.ucs03;
    const receiver = "0x15Ee7c367F4232241028c36E720803100757c6e9";
    const zkgm_token_minter = network?.escher?.tokenMinter;


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

        const eUContract = network?.contracts.cw20;
        let allowanceMsg = getExecuteAllowanceMsg(eUContract, userAddress, zkgm_token_minter, amount.toString());

        const quoteToken = network?.escher?.channel[destination_chain].stakedQuoteToken;
        if (!quoteToken || !network?.escher?.stakedBaseToken) {
            alert("no quote token or no native base token");
            return;
        }

        let tokenOrder =
            tokenOrderV2WithSolverMetadata(userAddress.toLowerCase(), recipient, network?.escher?.stakedBaseToken, amount, quoteToken as '0x${string}', amount, EU_FROM_UNION_SOLVER_METADATA_TESTNET);


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
                funds: []
            }),
        };


        try {
            const res = await client?.signAndBroadcast(userAddress, [allowanceMsg, executeTransferMsg], "auto", "transfer eu");
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
                        <div>
                            Note: To send to sepolia, after send the packet need to run curl to relay
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