"use client";


import { useState } from "react";
import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";

export default function QueryTotalBond() {
    const { userAddress, client, network } = useGlobalContext();
    const [totalBond, setTotalBond] = useState("");

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
        const msg = {
            bond: {}
        };

        console.log(JSON.stringify(msg));
        try {
            const msg = {
                total_bond_amount: {
                    delegator: "union1vnglhewf3w66cquy6hr7urjv3589srheampz42",
                    denom: "muno",
                    validators: ["unionvaloper1ex50uacwuhhgtu3pxm6w9znehkrw74s3x2l4lk"],
                }
            };
            const lst_contract = "union1h6efy80ax2d362zsuexhjy34gqnlru5neuhgegv2ed33ma8ge63s9le8v0"
            const res = await client?.queryContractSmart(
                lst_contract,
                msg
            );
            alert(JSON.stringify(res));
            setTotalBond(res?.amount);

        } catch (err) {
            console.log(err);
        }
    };

    return (
        <div className="w-full flex flex-row gap-4">
            <form onSubmit={handleSubmit}>
                <Card>
                    <CardBody className="gap-4">
                        <div>Total Bond: {totalBond}</div>
                    </CardBody>
                    <CardFooter>
                        <Button type="submit">Submit</Button>
                    </CardFooter>
                </Card>
            </form>
        </div>
    );
}
