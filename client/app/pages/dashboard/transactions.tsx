"use client";


import { Card, CardBody, CardHeader, Divider } from "@nextui-org/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";
import { IndexedTx } from "@cosmjs/stargate";

BigInt.prototype.toJSON = function () {
    return Number(this);
};

export default function Transactions() {

    const [bondTxs, setBondTxs] = useState<IndexedTx[] | undefined>([]);

    const {
        client,
        userAddress,
        network
    } = useGlobalContext();


    useEffect(() => {
        const getBondTxs = async () => {

            if (!userAddress) {
                return;
            }
            let txs = await client?.searchTx([{ key: "wasm-bond.sender", value: userAddress }]);
            console.log("Total bond: " + txs?.length);
        }

        getBondTxs();
    }, []);


    return (
        bondTxs && bondTxs.length > 0 ?
            <Card className="w-full flex mt-6">
                <CardHeader className="text-lg">Unbonding Records</CardHeader>
                <Divider />
                <CardBody className="gap-4">
                    <div className="flex flex-row text-sm">
                        {JSON.stringify(bondTxs)}
                    </div>
                </CardBody>
            </Card>
            : <></>
    );
}