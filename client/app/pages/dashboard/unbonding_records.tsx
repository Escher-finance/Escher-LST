"use client";


import { Card, CardBody, CardHeader, Divider } from "@heroui/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";

export default function UnbondingRecords() {

    const [unreleasedUnbondingRecords, setUnreleasedUnbondingRecords] = useState<any[]>([]);
    const [releasedUnbondingRecords, setReleasedUnbondingRecords] = useState<any[]>([]);
    const [liquidity, setLiquidity] = useState<any>(null);

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



    useEffect(() => {
        const getBalance = async () => {
            const msg: any = {
                unbond_record: {
                    staker: userAddress,
                    released: false
                }
            };

            const records = await client?.queryContractSmart(
                network?.contracts.lst,
                msg
            );

            console.log("unreleased", JSON.stringify(records));
            setUnreleasedUnbondingRecords(records);


            const releasedMsg: any = {
                unbond_record: {
                    staker: userAddress,
                    released: true
                }
            };

            const releasedRecords = await client?.queryContractSmart(
                network?.contracts.lst,
                releasedMsg
            );

            console.log("released", JSON.stringify(releasedRecords));

            setReleasedUnbondingRecords(releasedRecords);
        }

        getBalance();
        getLiquidity();
    }, []);

    return (
        <Card className="w-full flex mt-6">
            <CardHeader className="text-lg">Unbonding Records</CardHeader>
            <Divider />
            <CardBody className="gap-4">
                {unreleasedUnbondingRecords.length > 0 &&
                    <><div className="text-lg">In process</div>
                        <table>
                            <thead>
                                <tr>
                                    <th>Unbond Amount</th>
                                    <th>Estimated Received Amount</th>
                                </tr>
                            </thead>
                            <tbody>
                                {unreleasedUnbondingRecords.map((record: any, idx: number) => {
                                    return (
                                        <><tr key={"unreleased" + idx}>
                                            <td>
                                                {record.amount} {network?.stakeCurrency.liquidStakingDenomDisplay}
                                            </td>
                                            <td>
                                                {Intl.NumberFormat('en-US').format(Math.floor(record.amount * liquidity?.exchange_rate))}
                                            </td>
                                        </tr></>
                                    )
                                })}

                            </tbody>
                        </table></>
                }
                <br />
            </CardBody>
        </Card>
    );
}