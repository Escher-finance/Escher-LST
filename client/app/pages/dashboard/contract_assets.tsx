"use client";


import { Card, CardBody, CardHeader, Divider } from "@nextui-org/react";
import { useState, useEffect } from "react";
import { useGlobalContext } from "@/app/core/context";

interface AssetsProps {
    stateKey: number;
}

export default function ContractAssets({ stateKey }: AssetsProps) {

    const [stakeBalance, setStakeBalance] = useState("");
    const [lstakeBalance, setLstakeBalance] = useState("0");

    const {
        client,
        userAddress,
        network
    } = useGlobalContext();

    const getNativeBalance = async () => {
        let bal = await client?.getBalance(network?.contracts.lst, "ubbn");
        if (bal) {
            setStakeBalance(bal.amount);
        }

    }
    const getBalance = async () => {
        const msg: any = {
            balance: {
                address: network?.contracts.lst
            }
        };

        const { balance } = await client?.queryContractSmart(
            network?.contracts.cw20,
            msg
        );

        setLstakeBalance(balance);
    }

    const loadBalance = async () => {
        getNativeBalance();
        getBalance();
    }

    useEffect(() => {
        loadBalance();
    }, [stateKey]);

    useEffect(() => {
        loadBalance();
    }, [userAddress]);

    return (
        <Card className="w-full flex">
            <CardHeader className="text-lg p-3">LST Contract Assets</CardHeader>
            <Divider />
            <CardBody className="gap-1">
                <div className="flex flex-col">
                    <div className="p-3 text-sm">
                        Native: {stakeBalance} stake
                    </div>
                    <div className="p-3 text-sm">
                        LSToken: {lstakeBalance} lqStake
                    </div>
                </div>
            </CardBody>
        </Card>
    );
}