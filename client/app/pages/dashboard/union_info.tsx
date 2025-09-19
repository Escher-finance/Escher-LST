"use client";


import { Card, CardBody, CardHeader, Divider } from "@heroui/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";



interface KeyProps {
    stateKey: number;
}

export default function UnionInfo({ stateKey }: KeyProps) {

    const [liquidity, setLiquidity] = useState<any>(null);
    const [balance, setBalance] = useState("0");
    const [eUbalance, seteUBalance] = useState("0");

    const {
        client,
        userAddress,
        network
    } = useGlobalContext();


    const getLiquidity = async () => {
        const msg: any = {
            accounting_state: {}
        };

        const liquidity = await client?.queryContractSmart(
            network?.contracts.lst,
            msg
        );

        setLiquidity(liquidity);
    }


    const getBalance = async () => {
        if (userAddress) {
            const balance = await client?.getBalance(
                userAddress,
                network?.stakeCurrency.coinMinimalDenom
            );
            if (balance?.amount) {
                setBalance(balance?.amount);
            }

            let msg = {
                balance: {
                    address: userAddress
                }
            }
            const eUBalance = await client?.queryContractSmart(network?.contracts.cw20,
                msg
            );
            console.log(JSON.stringify(eUBalance));
            seteUBalance(eUBalance.balance);
        }
    }


    useEffect(() => {
        getLiquidity();
        getBalance();
    }, []);

    useEffect(() => {
        getLiquidity();
        getBalance();
    }, [stateKey]);


    return (
        <Card className="w-full flex">
            <CardHeader className="text-xl">Liquidity</CardHeader>
            <Divider />
            <CardBody className="gap-4">
                <div className="flex flex-row">
                    <div className="grid grid-cols-2 gap-1">
                        {liquidity &&
                            <>
                                <div>
                                    Purchase Rate
                                </div>
                                <div>
                                    {liquidity.purchase_rate}
                                </div>
                                <div>
                                    Redemption Rate
                                </div>
                                <div>
                                    {liquidity.redemption_rate}
                                </div>
                                <div>
                                    User Balance:
                                </div>
                                <div>
                                    {Number(balance) / (1e18)} {network?.stakeCurrency.coinDenom}
                                    <br />
                                    {Number(eUbalance) / (1e18)} {network?.stakeCurrency.liquidStakingDenomDisplay}
                                </div>

                            </>
                        }
                    </div>
                </div>
            </CardBody>
        </Card>
    );
}