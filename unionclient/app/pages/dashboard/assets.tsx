"use client";


import { Button, Card, CardBody, CardHeader, Divider } from "@nextui-org/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";

interface AssetsProps {
    stateKey: number;
}

export default function Assets({ stateKey }: AssetsProps) {

    const [stakeBalance, setStakeBalance] = useState("");
    const [lstakeBalance, setLstakeBalance] = useState("0");
    const faucetURL = "http://lstfaucet.rickyanto.com/";

    const {
        client,
        userAddress,
        network
    } = useGlobalContext();

    const getNativeBalance = async () => {
        if (!userAddress) {
            return;
        }
        let bal = await client?.getBalance(userAddress, network?.stakeCurrency.coinMinimalDenom);
        if (bal) {
            setStakeBalance(bal.amount);
        }
    }

    const getBalance = async () => {
        if (!userAddress) {
            return;
        }

        let msg = {
            balance: {
                address: userAddress
            }
        };
        const bal = await client?.queryContractSmart(network?.contracts.cw20, msg);

        if (bal) {
            setLstakeBalance(bal.balance);
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


    const faucetRequest = async () => {
        const msg = {
            address: userAddress,
            coins: ["100000muno"]
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
            <CardHeader className="text-lg p-3 gap-5">My Assets</CardHeader>
            <Divider />
            <CardBody className="gap-1">
                <div className="flex flex-col">
                    <div className="p-3 text-sm">
                        Native: {Intl.NumberFormat('en-US').format(Number(stakeBalance))} {network?.stakeCurrency.coinMinimalDenom}
                    </div>
                    <div className="p-3 text-sm">
                        LSToken: {Intl.NumberFormat('en-US').format(Number(lstakeBalance))} {network?.stakeCurrency.liquidStakingDenomDisplay}
                    </div>
                </div>
            </CardBody>
        </Card>
    );
}