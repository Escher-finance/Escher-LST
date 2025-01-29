"use client";


import { Card, CardBody, CardHeader, Divider } from "@nextui-org/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";

interface AssetsProps {
    stateKey: number;
}

export default function RevenueAssets({ stateKey }: AssetsProps) {


    const [stakeBalance, setStakeBalance] = useState("");
    const [lstakeBalance, setLstakeBalance] = useState("0");

    const {
        client,
        userAddress,
        network
    } = useGlobalContext();

    const getNativeBalance = async () => {
        let bal = await client?.getBalance(network?.contracts.reward, network?.stakeCurrency.coinMinimalDenom);
        if (bal) {
            setStakeBalance(bal.amount);
        }

    }
    const getBalance = async () => {
        const bal = await client?.getBalance(network?.contracts.reward, network?.stakeCurrency.liquidStakingDenom);

        if (bal) {
            setLstakeBalance(bal.amount);
        }
    }

    const loadBalance = async () => {
        getNativeBalance();
        getBalance();
    }


    useEffect(() => {
        loadBalance();
    }, [stateKey]);

    useEffect(() => {
        loadBalance();
    }, [userAddress]);

    return (
        <Card className="w-full flex">
            <CardHeader className="text-lg p-3">Reward Contract Assets</CardHeader>
            <Divider />
            <CardBody className="gap-1">
                <div className="flex flex-col">
                    <div className="p-3 text-sm">
                        Native: {stakeBalance} muno
                    </div>
                    <div className="p-3 text-sm">
                        LSToken: {lstakeBalance} limuno
                    </div>
                </div>
            </CardBody>
        </Card>
    );
}