"use client";

import { Card, CardBody, CardFooter, Button, Input } from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";

export default function TransferEVM() {
    const { userAddress, client } = useGlobalContext();

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
        if (!userAddress) {
            return;
        }
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const amount = formEntries.amount.toString();
        const denom = formEntries.denom.toString();
        const lst_contract = formEntries.lst_contract.toString();
        const msg = {
            transfer: {
                amount: {
                    amount,
                    denom,
                },
                receiver: "15Ee7c367F4232241028c36E720803100757c6e9",
            },
        };

        console.log(JSON.stringify(msg));
        try {
            const res = await client?.execute(
                userAddress,
                lst_contract,
                msg,
                "auto",
                "execute bond",
                [],
            );
            alert(res?.transactionHash);
        } catch (err) {
            console.log(err);
        }
    };

    return (
        <div className="w-full flex flex-row gap-4">
            <form onSubmit={handleSubmit} className="w-full">
                <Card className="w-full">
                    <CardBody className="gap-4">
                        <Input
                            isRequired
                            name="amount"
                            label="Amount"
                            defaultValue="10000"
                        />
                        <Input
                            isRequired
                            name="denom"
                            label="Denom"
                            defaultValue="factory/union1vnglhewf3w66cquy6hr7urjv3589srheampz42/stmomo"
                        />
                        <Input
                            isRequired
                            name="lst_contract"
                            label="LiquidStaking Contract"
                            defaultValue="union15zv347kq0n3cdfjhp2ez75mufdljjfyyp36c08hekdxu7zjnrvzs23rx7w"
                        />
                    </CardBody>
                    <CardFooter>
                        <Button type="submit">Submit</Button>
                    </CardFooter>
                </Card>
            </form>
        </div>
    );
}
