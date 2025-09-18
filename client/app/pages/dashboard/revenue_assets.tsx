"use client";


import { Button, Card, CardBody, CardHeader, Divider } from "@heroui/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";


interface KeyProps {
    stateKey: number;
}

export default function RevenueAssets({ stateKey }: KeyProps) {

    const [stakeBalance, setStakeBalance] = useState("");
    const faucetURL = "http://lstfaucet.rickyanto.com/";

    const {
        client,
        userAddress,
        network
    } = useGlobalContext();
    const receiver = network?.chainName == "uniontestnet" ? "union17z2ea0dtzkpu9lc2eh0jcwxywh40th5e0xla5q" : "bbn17z2ea0dtzkpu9lc2eh0jcwxywh40th5ej00y9g";

    const getNativeBalance = async () => {
        let bal = await client?.getBalance(receiver, network?.stakeCurrency.coinMinimalDenom);
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
            <CardHeader className="text-lg p-3 gap-5">Revenue Receiver </CardHeader>
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