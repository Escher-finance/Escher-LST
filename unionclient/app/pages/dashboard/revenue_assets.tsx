"use client";


import { Button, Card, CardBody, CardHeader, Divider } from "@nextui-org/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";


interface KeyProps {
    stateKey: number;
}

export default function RevenueAssets({ stateKey }: KeyProps) {

    const [stakeBalance, setStakeBalance] = useState("");
    const [lstakeBalance, setLstakeBalance] = useState("0");
    const faucetURL = "http://lstfaucet.rickyanto.com/";

    const receiver = "union1vnglhewf3w66cquy6hr7urjv3589srheampz42";
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

    const getBalance = async () => {
        const msg: any = {
            balance: {
                address: receiver
            }
        };

        const { balance } = await client?.queryContractSmart(
            network?.contracts.cw20,
            msg
        );

        setLstakeBalance(balance);
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


    const faucetRequest = async () => {
        const msg = {
            address: userAddress,
            coins: ["100000stake"]
        };
        console.log(msg);
        const response = await fetch(faucetURL, {
            method: "POST",
            body: JSON.stringify(msg),
            headers: {
                "Content-type": "application/json; charset=UTF-8",
            },
        });

        console.log(JSON.stringify(response.body));
        getNativeBalance();
    }

    return (
        <Card className="w-full flex">
            <CardHeader className="text-lg p-3 gap-5">Revenue Receiver </CardHeader>
            <Divider />
            <CardBody className="gap-1">
                <div className="flex flex-col">
                    <div className="p-3 text-sm">
                        Native: {stakeBalance} stake
                    </div>
                    <div className="p-3 text-sm">
                        LSToken: {lstakeBalance} lqStake
                    </div>
                </div>
            </CardBody>
        </Card>
    );
}