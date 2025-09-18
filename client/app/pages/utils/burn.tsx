"use client";

import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
} from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";

export default function Burn() {
    const { userAddress, client, network } = useGlobalContext();

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
        if (!userAddress) {
            alert("no user");
            return;
        }
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const lst_contract = network?.contracts.lst;
        const amount = formEntries.amount.toString();
        const msg = {
            burn: {
                amount
            }
        };

        console.log(JSON.stringify(msg));
        try {
            const res = await client?.execute(userAddress, lst_contract, msg, "auto", "execute bond", []);
            alert(res?.transactionHash);

        } catch (err) {
            console.log(err);
        }
    };

    return (
        <div className="w-full flex">
            <form onSubmit={handleSubmit} className="w-full flex">
                <Card className="grow">
                    <CardBody className="w-full gap-4">
                        <Input
                            isRequired
                            name="amount"
                            label="amount"
                            defaultValue="0"
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
