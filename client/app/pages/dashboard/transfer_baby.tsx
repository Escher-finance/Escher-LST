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
import { getSalt } from "@/app/lib/utils";
import { useState } from "react";

(BigInt.prototype as any).toJSON = function () {
    return this.toString();
};

export const chains = [
    { key: "1", label: "Sepolia" },
    { key: "2", label: "Holesky" },
];


interface KeyProps {
    stateKey: number;
    setStateKey: (key: number) => void;
}

export default function TransferBaby({ stateKey, setStateKey }: KeyProps) {
    const { userAddress, client, network } = useGlobalContext();
    const [isLoading, setIsLoading] = useState(false);

    const lst_contract = network?.contracts.lst;

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


        let msg = {
            transfer: {
                amount: amount.toString(),
                to: recipient,
                salt: getSalt(),
                channel_id: Number(destination_chain),
            },
        };

        console.log(JSON.stringify(msg));
        try {
            const res = await client?.execute(userAddress, lst_contract, msg, "auto", "transfer from babylon", [{
                amount: amount.toString(),
                denom: "ubbn"
            }]);
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
                            defaultValue="0x15Ee7c367F4232241028c36E720803100757c6e9"
                        />
                        <Select className="max-w-xs" label="Select destination chain" variant="flat" name="destination_chain">
                            {chains.map((chain) => (
                                <SelectItem key={chain.key}>{chain.label}</SelectItem>
                            ))}
                        </Select>

                        <div className="text-sm italic p-1">
                            Note: To send to sepolia and holesky, after send the packet need to run curl to relay (see README at client folder for CURL example)
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