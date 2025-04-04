"use client";

import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
    user,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";

export default function InstantiateDebugger() {
    const { userAddress, client, network } = useGlobalContext();

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
        if (!userAddress) {
            return;
        }

        let bal = await client?.getBalance(userAddress, network?.stakeCurrency.coinMinimalDenom);
        if (bal) {
            console.log(bal.amount);
        }
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const code_id = Number(formEntries.code_id);


        const msg = {};

        console.log(JSON.stringify(msg));
        try {
            const instantiateOptions = {
                memo: "Instantiating a new contract",
                funds: [],
                admin: userAddress,
            };

            const instantiateResult = await client?.instantiate(
                userAddress,
                code_id,
                msg,
                "reward",
                "auto",
                instantiateOptions
            );
            console.log(instantiateResult?.contractAddress);
            alert(instantiateResult?.contractAddress);

        } catch (err) {
            console.log(err);
        }
    };

    return (
        <div className="w-full flex flex-row gap-4">
            <form onSubmit={handleSubmit}>
                <Card>
                    <CardBody className="gap-4">
                        <Input
                            isRequired
                            name="code_id"
                            label="Debug CodeID"
                            className="max-w-xs"
                            defaultValue="93"
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
