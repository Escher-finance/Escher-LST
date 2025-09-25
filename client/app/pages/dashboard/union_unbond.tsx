

"use client";

import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
} from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";
import { useState } from "react";
import { MsgExecuteContract } from "cosmjs-types/cosmwasm/wasm/v1/tx";
import { toUtf8 } from "@cosmjs/encoding";

interface KeyProps {
    stateKey: number;
    setStateKey: (key: number) => void;
}

export default function UnionUnbond({ stateKey, setStateKey }: KeyProps) {
    const { userAddress, client, network } = useGlobalContext();

    const [isLoading, setIsLoading] = useState(false);

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

    const handleSubmit = async (e: any) => {

        e.preventDefault();
        if (!userAddress) {
            alert("no user");
            return;
        }



        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const amount = formEntries.amount.toString();


        const unbond_msg = {
            unbond: {
                amount: amount,
            },
        };

        if (Number(amount) < 1000) {
            alert("Sorry, minimal bond amount is 1000000");
            return;
        }

        let allowanceMsg = getExecuteAllowanceMsg(network?.contracts.cw20, userAddress, network?.contracts.lst, amount.toString());
        const executeUnbondMsg = {
            typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
            value: MsgExecuteContract.fromPartial({
                sender: userAddress,
                contract: network?.contracts.lst,
                msg: toUtf8(JSON.stringify(unbond_msg)),
                funds: []
            }),
        };

        try {
            setIsLoading(true);
            let msgs = [allowanceMsg, executeUnbondMsg];
            const res = await client?.signAndBroadcast(userAddress, msgs, "auto", "unbond");
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
                            defaultValue="1000000000000000000"
                        />
                    </CardBody>
                    <CardFooter>
                        <Button type="submit" isLoading={isLoading}>Unbond</Button>
                    </CardFooter>
                </Card>
            </form>
        </div>
    );
}
