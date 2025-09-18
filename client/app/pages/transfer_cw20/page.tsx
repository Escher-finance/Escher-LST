"use client";
import { useState } from "react";

import { Button, Input, Form, SelectItem, Select } from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";
import { getRecommendedChannels, getChannelInfo, getQuoteToken } from "@/app/lib/ucs03_helpers";
import { getSalt } from "@/app/lib/utils";
import { toHex } from "viem";
import { fromHex } from "viem";

const chains = [
    { key: "bbn-test-5", label: "Babylon" },
];

const CW20_BASE_TOKEN = "union1d0g6z2977xa6c5eknf78urltxx3tnvtjrq4c7fh99rpd5j4ut76qwf8r20"; // thisis CW20 base token for denom "funny"
const SOURCE_CHAIN_ID = "union-testnet-9";

function sleep(ms: number) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

export default function TransferCW20() {

    const [action, setAction] = useState<string | null>(null);
    const {
        client,
        userAddress,
    } = useGlobalContext();


    const transfer = async (ucs03Address: string, channelId: number, amount: string, baseToken: string, receiver: string, quoteToken: string) => {
        if (!userAddress) {
            return;
        }

        let allowance_msg = {
            increase_allowance: { spender: ucs03Address, amount: amount.toString() }
        };

        // let result1 = await client?.execute(userAddress, CW20_BASE_TOKEN, allowance_msg, "auto");
        // console.log(result1?.transactionHash);

        const msg = {
            transfer: {
                channel_id: channelId,
                receiver,
                base_token: baseToken,
                base_amount: amount,
                quote_token: quoteToken,
                quote_amount: amount,
                timeout_height: 1000000000,
                timeout_timestamp: 0,
                salt: getSalt(),
            }
        };
        console.log(JSON.stringify(msg));

        sleep(5);

        let result2 = await client?.execute(userAddress, ucs03Address, msg, "auto", `send ${amount} ${CW20_BASE_TOKEN} to ${receiver}`, [{ amount: amount.toString(), denom: CW20_BASE_TOKEN }]);

        return result2
    }


    const transferToken = async (e: any) => {
        e.preventDefault();
        let formData = new FormData(e.currentTarget);
        let data = Object.fromEntries(formData);

        let channels = await getRecommendedChannels();
        console.log("find source : {}", SOURCE_CHAIN_ID);
        console.log("find destination : {}", data.chain_id.toString());
        const channel = getChannelInfo(SOURCE_CHAIN_ID, data.chain_id.toString(), channels)
        if (channel === null) {
            console.log("no channel found");
            return;
        }
        console.log(JSON.stringify(channel));

        //const baseToken = toHex(CW20_BASE_TOKEN)
        const baseToken = toHex(CW20_BASE_TOKEN)
        let recipient = toHex(data.receiver.toString());
        const quoteToken = await getQuoteToken(SOURCE_CHAIN_ID, baseToken, channel)
        if (quoteToken.isErr()) {
            console.info("could not get quote token")
            console.error(quoteToken.error)
            return;
        }

        if (quoteToken.value.type === "NO_QUOTE_AVAILABLE") {
            console.error("No quote token available")
            return;
        }
        console.info("quote token", quoteToken.value)

        const ucs03Address = fromHex(`0x${channel.source_port_id}`, "string");
        console.log("ucs03Address", ucs03Address);
        let result = await transfer(ucs03Address, channel.source_channel_id, data.amount.toString(), CW20_BASE_TOKEN, recipient, quoteToken.value.quote_token);
        setAction(`Transfer is successful, Transaction hash: ${result?.transactionHash}`);
    }


    return (
        <div className="w-full flex flex-col gap-2 p-4">
            <div className="w-full">
                <h1 className="text-lg">Transfer CW20</h1>
                <div className="flex flex-col gap-4 p-4">
                    <div className="flex flex-col gap-4 text-left">
                        <div className="text-sm">
                            Sender: {userAddress}
                        </div>
                        <Form
                            className="w-full max-w-xs flex flex-col gap-4"
                            validationBehavior="native"
                            onSubmit={transferToken}
                        >

                            <Input
                                endContent={
                                    <div className="pointer-events-none flex items-center">
                                        <span className="text-default-400 text-small"> funny</span>
                                    </div>
                                }
                                label="Amount"
                                placeholder="0"
                                name="amount"
                                defaultValue="10"
                                isRequired
                            />
                            <Input
                                label="To"
                                placeholder="address"
                                name="receiver"
                                isRequired
                                defaultValue="bbn1vnglhewf3w66cquy6hr7urjv3589srheqj3myz"
                            />
                            <Select className="max-w-xs" label="Target chain" name="chain_id" isRequired defaultSelectedKeys={["bbn-test-5"]}>
                                {chains.map((chain) => (
                                    <SelectItem key={chain.key}>{chain.label}</SelectItem>
                                ))}
                            </Select>

                            <div className="flex gap-2">
                                <Button color="primary" type="submit">
                                    Submit
                                </Button>
                                <Button type="reset" variant="flat">
                                    Reset
                                </Button>
                            </div>
                            {action && (
                                <div className="text-small text-default-500">
                                    Action: <code>{action}</code>
                                </div>
                            )}
                        </Form>
                    </div>

                </div>

            </div>
        </div>
    );
}
