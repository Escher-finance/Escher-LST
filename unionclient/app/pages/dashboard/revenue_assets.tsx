"use client";


import { Button, Card, CardBody, CardHeader, Divider } from "@nextui-org/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";


interface KeyProps {
    stateKey: number;
}

export default function RevenueAssets({ stateKey }: KeyProps) {

    const [stakeBalance, setStakeBalance] = useState("");
    const faucetURL = "http://lstfaucet.rickyanto.com/";

    const receiver = "union17z2ea0dtzkpu9lc2eh0jcwxywh40th5e0xla5q";
    const {
        client,
        userAddress,
        network
    } = useGlobalContext();

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
                        Native: {Intl.NumberFormat('en-US').format(Number(stakeBalance))} muno
                    </div>

                </div>
            </CardBody>
        </Card>
    );
}