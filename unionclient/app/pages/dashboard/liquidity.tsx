"use client";


import { Card, CardBody, CardHeader, Divider } from "@nextui-org/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";


interface AssetsProps {
    stateKey: number;
}

export default function Liquidity({ stateKey }: AssetsProps) {

    const [liquidity, setLiquidity] = useState<any>(null);
    const [state, setState] = useState<any>(null);

    const {
        client,
        userAddress,
        network
    } = useGlobalContext();


    const getLiquidity = async () => {
        const msg: any = {
            staking_liquidity: {}
        };

        const liquidity = await client?.queryContractSmart(
            network?.contracts.lst,
            msg
        );

        setLiquidity(liquidity);
    }

    const getState = async () => {
        const msg: any = {
            state: {}
        };

        const state = await client?.queryContractSmart(
            network?.contracts.lst,
            msg
        );

        setState(state);
    }


    useEffect(() => {
        getLiquidity();
        getState();
    }, []);

    useEffect(() => {
        getLiquidity();
        getState();
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
                                    Current Exchange Rate:
                                </div>
                                <div>
                                    {liquidity.exchange_rate}
                                </div>
                                <div>
                                    Total Value:
                                </div>
                                <div>
                                    {liquidity.amount} {network?.stakeCurrency.coinMinimalDenom}
                                </div>
                                <div>
                                    Delegated:
                                </div>
                                <div>
                                    {liquidity.delegated} {network?.stakeCurrency.coinMinimalDenom}
                                </div>
                                <div>
                                    Reward:
                                </div>
                                <div>
                                    {liquidity.reward} {network?.stakeCurrency.coinMinimalDenom}
                                </div>
                                <div>
                                    Total Supply (Liquid Staking Token):
                                </div>
                                <div>
                                    {state?.total_supply} {network?.stakeCurrency.liquidStakingDenomDisplay} (lst contract) <br />
                                </div>
                                <div>
                                    Bond counter:
                                </div>
                                <div>
                                    {state?.bond_counter} times
                                </div>
                                <div>
                                    Liquidity Time:
                                </div>
                                <div>
                                    {(new Date(Number(liquidity?.time / 1000000)).toString())}
                                </div>
                            </>
                        }
                    </div>
                </div>
            </CardBody>
        </Card>
    );
}