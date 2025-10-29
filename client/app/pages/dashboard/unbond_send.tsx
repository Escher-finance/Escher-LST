"use client";

import { Card, CardBody, CardFooter, Button, Input } from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";
import { getExecuteContractMessage } from "@/utils/msg";
import { useState } from "react";
import { toHex } from "viem";
import { toBase64 } from "@cosmjs/encoding";

interface KeyProps {
    stateKey: number;
    setStateKey: (key: number) => void;
}

export default function UnbondViaSend({ stateKey, setStateKey }: KeyProps) {
    const { userAddress, client, network } = useGlobalContext();
    const [isLoading, setIsLoading] = useState(false);

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
        setIsLoading(true);

        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const amount = formEntries.amount.toString();
        const recipient = formEntries.recipient.toString().trim();
        const recipient_channel_id =
            formEntries.recipient_channel_id.toString();
        //const recipient_ibc_channel_id = formEntries.recipient_ibc_channel_id.toString();
        const encoder = new TextEncoder();
        const recipient_hex = toHex(encoder.encode(recipient));

        const msg: any = {
            staking_liquidity: {},
        };

        const liquidity = await client?.queryContractSmart(
            network?.contracts.lst,
            msg,
        );

        let undelegate_amount = Number(amount) * liquidity.exchange_rate;

        let max_amount = Math.floor(
            liquidity.delegated / liquidity.exchange_rate,
        );
        if (undelegate_amount >= liquidity.delegated) {
            alert(
                "Not enough fund to be undelegated, please reduce your unbonding amount to below < " +
                    max_amount.toString(),
            );
            setIsLoading(false);
            return;
        }

        try {
            if (!userAddress) {
                alert("no user wallet");
                setIsLoading(false);
                return;
            }

            // Define the Unbonding payload
            const unbondingPayload = {
                unstake: {
                    amount,
                    recipient:
                        recipient_channel_id == "0"
                            ? recipient
                            : recipient.indexOf("0x") == -1
                              ? recipient_hex
                              : recipient,
                    recipient_channel_id:
                        recipient_channel_id == "0"
                            ? null
                            : Number(recipient_channel_id),
                    recipient_ibc_channel_id: null,
                },
            };

            // Encode the payload as base64 (Binary)
            const payloadJson = JSON.stringify(unbondingPayload);

            console.log(payloadJson);
            const payloadBytes = new TextEncoder().encode(payloadJson); // Convert string to Uint8Array
            const payloadBinary = toBase64(payloadBytes); // Convert Uint8Array to base64 string

            const unbondingMsg = {
                send: {
                    contract: network?.contracts.lst,
                    amount: amount, // Amount of tokens to send
                    msg: payloadBinary, // Encoded Cw20PayloadMsg payload
                },
            };
            console.log(JSON.stringify(unbondingMsg));

            // send to cw20 contract
            const executeUnbondingMsg = getExecuteContractMessage(
                userAddress,
                network?.contracts.cw20,
                unbondingMsg,
                [],
            );

            let msgs = [executeUnbondingMsg];
            const res = await client?.signAndBroadcast(
                userAddress,
                msgs,
                "auto",
                "",
            );
            alert(res?.transactionHash);
            console.log(res?.transactionHash);

            let newKey = stateKey + 1;
            setStateKey(newKey);
            setIsLoading(false);
        } catch (err) {
            setIsLoading(false);
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
                            defaultValue="10000"
                        />
                        <Input
                            isRequired
                            name="recipient"
                            label="Recipient (example: bbn1vnglhewf3w66cquy6hr7urjv3589srheqj3myz or 0x15Ee7c367F4232241028c36E720803100757c6e9 for other chain)"
                            defaultValue="bbn1vnglhewf3w66cquy6hr7urjv3589srheqj3myz"
                        />
                        <Input
                            name="recipient_channel_id"
                            label="Recipient Channel ID (1 for sepolia, 2 for holesky)"
                            defaultValue="0"
                        />
                    </CardBody>
                    <CardFooter>
                        <Button type="submit" isLoading={isLoading}>
                            Unbond
                        </Button>
                    </CardFooter>
                </Card>
            </form>
        </div>
    );
}
