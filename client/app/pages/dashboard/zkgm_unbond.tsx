"use client";

import { Card, CardBody, CardFooter, Button, Input } from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";
import { MsgExecuteContract } from "cosmjs-types/cosmwasm/wasm/v1/tx";
import { toUtf8 } from "@cosmjs/encoding";
import {
    BYTECODE_BASE_CHECKSUM,
    encodeInstruction,
    getAddressFromEvm,
    MODULE_HASH,
    unbondSendToIBC,
} from "@/app/lib/ucs03";
import { Instruction } from "@unionlabs/sdk/Ucs03";
import { getSalt } from "@/app/lib/utils";
import { useState } from "react";
import { getTimeoutInNanoseconds7DaysFromNow } from "@/app/lib/utils";
import { Effect } from "effect";
import { toHex } from "viem";
import { ChannelId } from "@unionlabs/sdk/schema/channel";
import Networks from "@/config/networks.config";

interface KeyProps {
    stateKey: number;
    setStateKey: (key: number) => void;
}

export default function ZkgmUnbond({ stateKey, setStateKey }: KeyProps) {
    const [isLoading, setIsLoading] = useState(false);

    const { userAddress, client, network } = useGlobalContext();
    const ucs03_contract = network?.escher.ucs03;
    const channel_id = network?.escher.channel["babylon"]?.sourceChannelId;
    const destination_channel_id =
        network?.escher.channel["babylon"]?.destinationChannelId;
    const targetChain =
        network?.chainName.indexOf("testnet") != -1
            ? "babylon-testnet"
            : "babylon-mainnet";

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

        console.log("destination_channel_id", destination_channel_id);
        console.log("ucs03", Networks[targetChain].escher.ucs03);

        const proxyAddress = await Effect.runPromise(
            getAddressFromEvm({
                path: BigInt(0),
                channel: ChannelId.make(destination_channel_id),
                sender: toHex(userAddress),
                ucs03: Networks[targetChain].escher
                    .ucs03 as `${string}1${string}`,
                bytecode_base_checksum: BYTECODE_BASE_CHECKSUM,
                module_hash: MODULE_HASH,
            }),
        );

        console.log("proxyAddress.address", proxyAddress.address);

        let callsInstruction = await unbondSendToIBC(
            userAddress,
            amount,
            proxyAddress.address,
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
                funds: [
                    {
                        amount: amount.toString(),
                        denom: network?.escher.ebabyDenom,
                    },
                ],
            }),
        };

        console.log(JSON.stringify(executeSendMsg));
        try {
            setIsLoading(true);
            const res = await client?.signAndBroadcast(
                userAddress,
                [executeSendMsg],
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
