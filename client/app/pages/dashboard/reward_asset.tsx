"use client";


import { Card, CardBody, CardHeader, Divider } from "@nextui-org/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";

interface AssetsProps {
    stateKey: number;
}

export default function RevenueAssets({ stateKey }: AssetsProps) {


    const [stakeBalance, setStakeBalance] = useState("");

    const {
        client,
        userAddress,
        network
    } = useGlobalContext();

    const getNativeBalance = async () => {

        let bal = network?.chainName == "uniontestnet" ? await client?.getBalance(network?.contracts.reward, network?.stakeCurrency.coinMinimalDenom) : await client?.queryContractSmart(network?.contracts.reward, { balance: {} });

        if (bal) {
            setStakeBalance(bal.amount);
        }

    }

    useEffect(() => {
        getNativeBalance();
    }, [stateKey]);

    useEffect(() => {
        getNativeBalance();
    }, [userAddress]);

    return (
        <Card className="w-full flex">
            <CardHeader className="text-lg p-3">Reward Contract</CardHeader>
            <Divider />
            <CardBody className="gap-1">
                <div className="flex flex-col">
                    <div className="p-3 text-sm">
                        Native: {Intl.NumberFormat('en-US').format(Number(stakeBalance))} {network?.stakeCurrency.coinMinimalDenom}
                    </div>
                </div>
            </CardBody>
        </Card>
    );
}