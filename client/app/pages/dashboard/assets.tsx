"use client";


import { Button, Card, CardBody, CardHeader, Divider } from "@nextui-org/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";

export default function Assets() {

    const [stakeBalance, setStakeBalance] = useState("");
    const [lstakeBalance, setLstakeBalance] = useState("0");
    const faucetURL = "http://100.42.181.110:4500/";

    const {
        client,
        userAddress,
        network
    } = useGlobalContext();

    const getNativeBalance = async () => {
        let bal = await client.getBalance(userAddress, "stake");
        setStakeBalance(bal.amount);
    }

    const getBalance = async () => {
        const msg: any = {
            balance: {
                address: userAddress
            }
        };

        const { balance } = await client.queryContractSmart(
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
            <CardHeader className="text-xl p-3 gap-5">Your Assets  <Button onPress={faucetRequest}>Faucet Request Token</Button></CardHeader>
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