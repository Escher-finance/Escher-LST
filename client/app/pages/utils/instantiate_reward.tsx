"use client";

import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";

export default function InstantiateReward() {
    const { userAddress, client, network } = useGlobalContext();

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const code_id = Number(formEntries.code_id);


        const msg = {
            coin_denom: "stake",
            fee_rate: "0.1",
            revenue_receiver: "cosmos1pss37x3hwq5ytk7uhf9fjcpcd7k20pekq6xtlz",
            lst_contract: network?.contracts.lst,
        };

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
                            label="Reward CodeID"
                            className="max-w-xs"
                            defaultValue="6"
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
