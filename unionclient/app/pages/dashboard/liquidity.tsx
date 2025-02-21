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
    const [totalSupply, setTotalSupply] = useState(0);

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

    const getTotalSupply = async () => {
        const msg: any = {
            token_info: {}
        };
        const balance = await client?.queryContractSmart(network?.contracts.cw20, msg);
        setTotalSupply(balance.total_supply);

    }


    useEffect(() => {
        getLiquidity();
        getState();
        getTotalSupply();
    }, []);

    useEffect(() => {
        getLiquidity();
        getState();
        getTotalSupply();
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
                                    {Intl.NumberFormat('en-US').format(Number(liquidity.amount))} {network?.stakeCurrency.coinMinimalDenom}
                                </div>
                                <div>
                                    Delegated:
                                </div>
                                <div>
                                    {Intl.NumberFormat('en-US').format(Number(liquidity.delegated))} {network?.stakeCurrency.coinMinimalDenom}
                                </div>
                                <div>
                                    Reward:
                                </div>
                                <div>
                                    {Intl.NumberFormat('en-US').format(Number(liquidity.reward))}  {network?.stakeCurrency.coinMinimalDenom}
                                </div>
                                <div>
                                    Total Supply (Liquid Staking Token):
                                </div>
                                <div>
                                    {Intl.NumberFormat('en-US').format(totalSupply)}  {network?.stakeCurrency.liquidStakingDenomDisplay}   (cw20) <br />
                                    {Intl.NumberFormat('en-US').format(Number(state?.total_supply))}  {network?.stakeCurrency.liquidStakingDenomDisplay} (lst contract)

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