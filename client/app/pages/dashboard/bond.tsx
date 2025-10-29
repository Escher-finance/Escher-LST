"use client";

import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
    SelectItem,
    Select,
} from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";
import { useState } from "react";
import { getSalt } from "@/app/lib/utils";
import { MsgExecuteContract } from "cosmjs-types/cosmwasm/wasm/v1/tx";
import { getTimeoutInNanoseconds7DaysFromNow } from "@/app/lib/utils";
import {
    encodeInstruction,
    encodeTokenOrderV2,
    tokenOrderV2Escrow,
} from "@/app/lib/ucs03";
import { Instruction } from "@unionlabs/sdk/Ucs03";
import { toUtf8 } from "@cosmjs/encoding";

interface KeyProps {
    stateKey: number;
    setStateKey: (key: number) => void;
}
const recipient_types = [
    { key: "on_chain", label: "On Chain" },
    { key: "zkgm", label: "Zkgm" },
];

export const chains = [
    { key: "sepolia", label: "Sepolia" },
    { key: "holesky", label: "Holesky" },
];

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

export default function Bond({ stateKey, setStateKey }: KeyProps) {
    const { userAddress, client, network } = useGlobalContext();

    const [isLoading, setIsLoading] = useState(false);
    const [selectedRecipientType, setSelectedRecipientType] = useState<
        string | undefined
    >(undefined);

    const handleSelectionChange = (e: any) => {
        setSelectedRecipientType(e.target.value);
    };

    const handleSubmit = async (e: any) => {
        e.preventDefault();
        if (!userAddress) {
            alert("no user");
            return;
        }

        const liquidity = await client?.queryContractSmart(
            network?.contracts.lst,
            {
                staking_liquidity: {},
            },
        );

        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const amount = formEntries.amount.toString();
        let recipient_type = formEntries.recipient_type.toString();
        let recipient_address = formEntries.address.toString();
        const chain_id = formEntries.chain_id?.toString();

        console.log("exchange rate", liquidity.exchange_rate);

        const min_mint_amount = Math.floor(
            (Number(amount) / Number(liquidity.exchange_rate)) * 0.995,
        );

        console.log("min_mint_amount", min_mint_amount);
        const bondMsg = {
            bond_v2: {
                min_mint_amount: min_mint_amount.toString(),
                mint_to_address: userAddress,
            },
        };

        console.log(bondMsg);

        const funds = [
            {
                amount,
                denom: network?.stakeCurrency.coinMinimalDenom,
            },
        ];

        const executeBondMsg = {
            typeUrl: "/cosmwasm.wasm.v1.MsgExecuteContract",
            value: MsgExecuteContract.fromPartial({
                sender: userAddress,
                contract: network?.contracts.lst,
                msg: toUtf8(JSON.stringify(bondMsg)),
                funds,
            }),
        };

        if (Number(amount) < 1000) {
            alert("Sorry, minimal bond amount is 1000000");
            return;
        }

        let msgs = [];
        console.log("recipient_type", selectedRecipientType);

        if (selectedRecipientType != "zkgm") {
            msgs = [executeBondMsg];
        } else {
            let baseToken = network?.escher?.stakedBaseToken;
            let quoteToken = chain_id
                ? network?.escher?.channel[chain_id].stakedQuoteToken
                : "";

            if (!baseToken) {
                alert("No base token");
                return;
            }

            let allowanceMsg = getExecuteAllowanceMsg(
                network?.contracts.cw20,
                userAddress,
                network?.escher?.tokenMinter,
                min_mint_amount.toString(),
            );

            let tokenOrder = tokenOrderV2Escrow(
                userAddress.toLowerCase(),
                recipient_address,
                baseToken,
                BigInt(min_mint_amount),
                quoteToken as "0x${string}",
            );
            let cosmos_msg = {
                send: {
                    channel_id:
                        network?.escher?.channel[chain_id]?.sourceChannelId,
                    timeout_height: "0",
                    timeout_timestamp:
                        getTimeoutInNanoseconds7DaysFromNow().toString(),
                    salt: getSalt(),
                    instruction: encodeInstruction(
                        Instruction.make({
                            opcode: 3,
                            version: 2,
                            operand: encodeTokenOrderV2(tokenOrder),
                        }),
                    ),
                },
            };
            const executeSendMsg = {
                typeUrl: "/cosmwasm.wasm.v1.MsgExecuteContract",
                value: MsgExecuteContract.fromPartial({
                    sender: userAddress,
                    contract: network?.escher?.ucs03,
                    msg: toUtf8(JSON.stringify(cosmos_msg)),
                    funds: [],
                }),
            };

            msgs = [executeBondMsg, allowanceMsg, executeSendMsg];
        }

        console.log(JSON.stringify(msgs));

        try {
            setIsLoading(true);
            const res = await client?.signAndBroadcast(
                userAddress,
                msgs,
                "auto",
                "bond from babylon",
            );
            alert(res?.transactionHash);
            let newKey = stateKey + 1;
            setStateKey(newKey);
            setIsLoading(false);
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
                            defaultValue="10000"
                        />
                        <Select
                            isRequired
                            className="max-w-xs"
                            label="Select Recipient"
                            variant="flat"
                            name="recipient_type"
                            defaultSelectedKeys={["on_chain"]}
                            onChange={handleSelectionChange}
                        >
                            {recipient_types.map((chain: any) => (
                                <SelectItem key={chain.key}>
                                    {chain.label}
                                </SelectItem>
                            ))}
                        </Select>
                        <Input
                            name="address"
                            label="Recipient address (example: bbn1vnglhewf3w66cquy6hr7urjv3589srheqj3myz / 0x15Ee7c367F4232241028c36E720803100757c6e9)"
                            defaultValue={userAddress ? userAddress : ""}
                        />
                        {selectedRecipientType &&
                            selectedRecipientType == "zkgm" && (
                                <Select
                                    className="max-w-xs"
                                    label="Select destination chain"
                                    variant="flat"
                                    name="chain_id"
                                    defaultSelectedKeys={["sepolia"]}
                                >
                                    {chains.map((chain) => (
                                        <SelectItem key={chain.key}>
                                            {chain.label}
                                        </SelectItem>
                                    ))}
                                </Select>
                            )}
                    </CardBody>
                    <CardFooter>
                        <Button type="submit" isLoading={isLoading}>
                            Bond
                        </Button>
                    </CardFooter>
                </Card>
            </form>
        </div>
    );
}

//bbn1fh0yyvuxz7l0vcusq5jc9zvzpm8ec2auvvkh44
//xion1vnglhewf3w66cquy6hr7urjv3589srhe496gds
