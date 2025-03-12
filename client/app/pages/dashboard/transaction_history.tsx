"use client";


import { Card, CardBody, CardHeader, Divider } from "@nextui-org/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";

export default function TransactionHistory() {

    const [records, setRecords] = useState<any | undefined>(undefined);

    const {
        client,
        userAddress,
        network,
        queryClient
    } = useGlobalContext();

    let explorer_url = network?.chainName == "uniontestnet" ? "https://explorer.testnet-9.union.build/union/tx/" : "https://testnet.babylon.explorers.guru/transaction/";

    useEffect(() => {
        const getTransactions = async () => {
            if (!userAddress || !queryClient) {
                setRecords([]);
                return;
            }
            console.log("before query transactions");
            const results = await queryClient.searchTx(
                `wasm.staker='${userAddress}' AND wasm.action='bond'  AND wasm._contract_address='${network?.contracts.lst}'`
            );


            let recs: any[] = [];
            if (results != undefined && results.length > 0) {

                for (var i = 0; i < results.length; i++) {
                    results[i].events.filter((r: any) => r.type == "wasm").forEach((ev: any) => {
                        let bond_amount = ev.attributes.find((e: any) => e.key == "bond_amount");
                        if (bond_amount) {
                            recs.push({ "height": results[i].height, "type": "bond", "amount": bond_amount.value, "hash": results[i].hash });
                        }

                    });
                }

            }


            const unbond_results = await queryClient.searchTx(
                "wasm.staker='" + userAddress + "' AND wasm.action='unbond' AND wasm._contract_address='" + network?.contracts.lst + "'"
            );

            if (unbond_results != undefined && unbond_results.length > 0) {

                for (var i = 0; i < unbond_results.length; i++) {
                    unbond_results[i].events.filter((r: any) => r.type == "wasm").forEach((ev: any) => {
                        let unbond_amount = ev.attributes.find((e: any) => e.key == "unbond_amount");
                        if (unbond_amount) {
                            recs.push({ "hash": unbond_results[i].hash, "height": unbond_results[i].height, "type": "unbond", "amount": unbond_amount.value });
                        }
                    });
                }

            }

            recs.sort(function (a, b) {
                return b.height - a.height
            });
            setRecords(recs);

        };

        getTransactions();
    }, []);

    return (
        <Card className="w-full flex mt-6">
            <CardHeader className="text-lg">Transaction History</CardHeader>
            <Divider />
            <CardBody className="gap-4">
                {records && records.map((r: any) => {
                    return <div>{r.type.toUpperCase()} {r.amount} {r.type == "bond" ? "muno" : "emuno"} at height: {r.height} <a href={`${explorer_url}${r.hash}`} target="_blank"> [Explorer]</a></div>
                })
                }
            </CardBody>
        </Card >
    );
}



// console.log(JSON.stringify(results, (key, value) =>
//     typeof value === "bigint" ? Number(value) : value,
// ));