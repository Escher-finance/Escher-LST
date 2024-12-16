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

            const records = await client.queryContractSmart(
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
                    <div className="flex flex-row text-sm">
                        {JSON.stringify(unbondingRecords)}
                    </div>
                </CardBody>
            </Card>
            : <></>
    );
}