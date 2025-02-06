"use client";


import { Card, CardBody, CardHeader, Divider } from "@nextui-org/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";

export default function UnbondingRecords() {

    const [unreleasedUnbondingRecords, setUnreleasedUnbondingRecords] = useState<any[]>([]);
    const [releasedUnbondingRecords, setReleasedUnbondingRecords] = useState<any[]>([]);

    const {
        client,
        userAddress,
        network
    } = useGlobalContext();


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

            setReleasedUnbondingRecords(releasedRecords);
        }

        getBalance();
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
                                    <th>Received Amount</th>
                                    <th>Rate</th>
                                    <th>Started</th>
                                    <th>Complete Estimation</th>
                                </tr>
                            </thead>
                            <tbody>
                                {unreleasedUnbondingRecords.map((record: any, idx: number) => {
                                    return (
                                        <><tr key={"unreleased" + idx}>
                                            <td>
                                                {record.amount.amount} limuno
                                            </td>
                                            <td>
                                                {record.undelegate_amount.amount} muno
                                            </td>
                                            <td>
                                                {parseFloat(record.exchange_rate).toFixed(2)}
                                            </td>
                                            <td>
                                                {new Date(Number(record.created / 1000000)).toLocaleString()}
                                            </td>
                                            <td>
                                                {new Date(Number(record.completion / 1000000) + 120000).toLocaleString()}
                                            </td>
                                        </tr></>
                                    )
                                })}

                            </tbody>
                        </table></>
                }
                <br />
                {releasedUnbondingRecords.length > 0 &&
                    <><div className="text-lg">RELEASED</div>
                        <table>
                            <thead>
                                <tr>
                                    <th>Unbond Amount</th>
                                    <th>Received Amount</th>
                                    <th>Rate</th>
                                    <th>Started</th>
                                    <th>Released</th>
                                </tr>
                            </thead>
                            <tbody>
                                {releasedUnbondingRecords.map((record: any, idx: number) => {
                                    return (
                                        <><tr key={idx}>
                                            <td>
                                                {record.amount.amount} limuno
                                            </td>
                                            <td>
                                                {record.undelegate_amount.amount} muno
                                            </td>
                                            <td>
                                                {parseFloat(record.exchange_rate).toFixed(2)}
                                            </td>
                                            <td>
                                                {new Date(Number(record.created / 1000000)).toLocaleString()}
                                            </td>
                                            <td>
                                                {new Date(Number(record.completion / 1000000) + 120000).toLocaleString()}
                                            </td>
                                        </tr></>
                                    )
                                })}

                            </tbody>
                        </table>
                    </>
                }
            </CardBody>
        </Card>
    );
}