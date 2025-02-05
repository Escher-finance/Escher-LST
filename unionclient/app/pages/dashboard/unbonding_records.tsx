"use client";


import { Card, CardBody, CardHeader, Divider } from "@nextui-org/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";

export default function UnbondingRecords() {

    const [unbondingRecords, setUnbondingRecords] = useState<any[]>([]);

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

            setUnbondingRecords(records);
        }

        getBalance();
    }, []);

    return (
        unbondingRecords.length > 0 ?
            <Card className="w-full flex mt-6">
                <CardHeader className="text-lg">Unbonding Records</CardHeader>
                <Divider />
                <CardBody className="gap-4">
                    <table>
                        <thead>
                            <tr>
                                <th>Unbond Amount</th>
                                <th>Received Amount</th>
                                <th>Rate</th>
                                <th>Started</th>
                                <th>Completion</th>
                            </tr>
                        </thead>
                        <tbody>
                            {unbondingRecords.map((record: any) => {
                                return (
                                    <><tr>
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
                </CardBody>
            </Card>
            : <div>None</div>
    );
}