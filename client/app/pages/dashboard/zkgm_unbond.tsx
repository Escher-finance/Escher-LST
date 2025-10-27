"use client";

import { Card, CardBody, CardFooter, Button, Input } from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";
import { MsgExecuteContract } from "cosmjs-types/cosmwasm/wasm/v1/tx";
import { toUtf8 } from "@cosmjs/encoding";
import { encodeInstruction, unbond } from "@/app/lib/ucs03";
import { Instruction } from "@unionlabs/sdk/Ucs03";
import { getSalt } from "@/app/lib/utils";
import { useState } from "react";
import { getTimeoutInNanoseconds7DaysFromNow } from "@/app/lib/utils";

interface KeyProps {
    stateKey: number;
    setStateKey: (key: number) => void;
}

const getExecuteAllowanceMsg = (
    contract: string,
    sender: string,
    spender: string,
    amount: string,
) => {
    let allowanceMsg = {
        increase_allowance: {
            spender,
            amount,
        },
    };
    console.log(JSON.stringify(allowanceMsg));
    const executeAllowanceMsg = {
        typeUrl: "/cosmwasm.wasm.v1.MsgExecuteContract",
        value: MsgExecuteContract.fromPartial({
            sender,
            contract,
            msg: toUtf8(JSON.stringify(allowanceMsg)),
            funds: [],
        }),
    };

    return executeAllowanceMsg;
};

export default function ZkgmUnbond({ stateKey, setStateKey }: KeyProps) {
    const [isLoading, setIsLoading] = useState(false);

    const { userAddress, client, network } = useGlobalContext();
    const ucs03_contract = network?.escher.ucs03;
    const channel_id = network?.escher.channel["babylon"]?.sourceChannelId;

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

        const allowanceMsg = getExecuteAllowanceMsg(
            ucs03_contract,
            userAddress,
            network?.escher?.tokenMinter,
            amount.toString(),
        );

        //todo: get proxyAddress here
        let proxyAddress = "osmo17z2ea0dtzkpu9lc2eh0jcwxywh40th5ed2d9vr";

        let callsInstruction = await unbond(
            userAddress,
            amount,
            proxyAddress,
            "babylon",
            network,
        );

        let sendMsg = {
            send: {
                channel_id,
                timeout_height: "0",
                timeout_timestamp:
                    getTimeoutInNanoseconds7DaysFromNow().toString(),
                salt: getSalt(),
                instruction: encodeInstruction(callsInstruction),
            },
        };

        const executeSendMsg = {
            typeUrl: "/cosmwasm.wasm.v1.MsgExecuteContract",
            value: MsgExecuteContract.fromPartial({
                sender: userAddress,
                contract: ucs03_contract,
                msg: toUtf8(JSON.stringify(sendMsg)),
                funds: [],
            }),
        };

        console.log(JSON.stringify(executeSendMsg));
        try {
            const res = await client?.signAndBroadcast(
                userAddress,
                [allowanceMsg, executeSendMsg],
                "auto",
                "unbond from osmosis to babylon",
            );
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
                    </CardBody>
                    <CardFooter>
                        <Button type="submit" isLoading={isLoading}>
                            Submit
                        </Button>
                    </CardFooter>
                </Card>
            </form>
        </div>
    );
}

//https://btc.union.build/explorer/packets/0xce8c32b71b5a7608b6b1afdea4fbb53c66cb026ed68916891d557277adbcfd4c
