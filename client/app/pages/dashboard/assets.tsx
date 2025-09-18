"use client";


import { Button, Card, CardBody, CardHeader, Chip, Divider, Tooltip } from "@heroui/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";

interface AssetsProps {
    stateKey: number;
}

export default function Assets({ stateKey }: AssetsProps) {

    const [stakeBalance, setStakeBalance] = useState("");

    const [ustakeBalance, setUStakeBalance] = useState("");

    const [lstakeBalance, setLstakeBalance] = useState("0");

    const [ulstakeBalance, setULStakeBalance] = useState("");


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

            setUStakeBalance((Number(bal.amount) / 1000000).toFixed(3))
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

            setULStakeBalance((Number(bal.balance) / 1000000).toFixed(3));
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

                        <span className="pr-3">Balance:</span>
                        <Tooltip
                            closeDelay={0}
                            content={Intl.NumberFormat('en-US').format(Number(stakeBalance)) + " " + network?.stakeCurrency.coinMinimalDenom}
                            delay={0}
                            motionProps={{
                                variants: {
                                    exit: {
                                        opacity: 0,
                                        transition: {
                                            duration: 0.1,
                                            ease: "easeIn",
                                        },
                                    },
                                    enter: {
                                        opacity: 1,
                                        transition: {
                                            duration: 0.15,
                                            ease: "easeOut",
                                        },
                                    },
                                },
                            }}
                        ><Chip>{Intl.NumberFormat('en-US').format(Number(ustakeBalance))} {network?.stakeCurrency.coinDenom}</Chip></Tooltip>
                        <br />

                    </div>
                    <div className="p-3 text-sm">
                        <span className="pr-3">LSToken:</span>
                        <Tooltip
                            closeDelay={0}
                            content={Intl.NumberFormat('en-US').format(Number(lstakeBalance)) + " " + network?.stakeCurrency.liquidStakingDenom}
                            delay={0}
                            motionProps={{
                                variants: {
                                    exit: {
                                        opacity: 0,
                                        transition: {
                                            duration: 0.1,
                                            ease: "easeIn",
                                        },
                                    },
                                    enter: {
                                        opacity: 1,
                                        transition: {
                                            duration: 0.15,
                                            ease: "easeOut",
                                        },
                                    },
                                },
                            }}
                        ><Chip>{Intl.NumberFormat('en-US').format(Number(ulstakeBalance))}  {network?.stakeCurrency.liquidStakingDenomDisplay}</Chip></Tooltip>

                    </div>
                </div>
            </CardBody>
        </Card >
    );
}