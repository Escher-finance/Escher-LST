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
    const [tokenInfo, setTokenInfo] = useState<any>(null);

    const {
        client,
        userAddress,
        network
    } = useGlobalContext();


    const getLiquidity = async () => {
        const msg: any = {
            staking_liquidity: {}
        };

        const liquidity = await client.queryContractSmart(
            network?.contracts.lst,
            msg
        );

        setLiquidity(liquidity);
    }

    const getState = async () => {
        const msg: any = {
            state: {}
        };

        const state = await client.queryContractSmart(
            network?.contracts.lst,
            msg
        );

        setState(state);
    }

    const getTokenInfo = async () => {
        const msg: any = {
            token_info: {}
        };

        const token_info = await client.queryContractSmart(
            network?.contracts.cw20,
            msg
        );

        setTokenInfo(token_info);
    }

    useEffect(() => {
        getLiquidity();
        getState();
        getTokenInfo();
    }, []);

    useEffect(() => {
        getLiquidity();
        getState();
        getTokenInfo();
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
                                    {liquidity.amount} stake
                                </div>
                                <div>
                                    Delegated:
                                </div>
                                <div>
                                    {liquidity.delegated} stake
                                </div>
                                <div>
                                    Reward:
                                </div>
                                <div>
                                    {liquidity.reward} stake
                                </div>
                                <div>
                                    Total Supply (Liquid Staking Token):
                                </div>
                                <div>
                                    {state?.total_lst_supply} lqStake (lst contract) <br />
                                    {tokenInfo?.total_supply} lqStake (cw20)
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
                                <div>
                                    Chain:
                                </div>
                                <div>
                                    {state?.chain}
                                </div>
                            </>
                        }
                    </div>
                </div>
            </CardBody>
        </Card>
    );
}