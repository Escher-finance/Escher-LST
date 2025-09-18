"use client";

import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
} from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";

export default function MoveToReward() {
    const { userAddress, client, network } = useGlobalContext();

    const handleSubmit = async (e: any) => {
        if (!userAddress) {
            alert("error no user");
            return;
        }
        // Prevent the browser from reloading the page
        e.preventDefault();
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const lst = formEntries.lst.toString();
        const msg = {
            move_to_reward: {}
        };

        console.log(JSON.stringify(msg));
        try {
            const res = await client?.execute(userAddress, lst, msg, "auto");
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
                            name="lst"
                            label="LST Contract"
                            defaultValue={network?.contracts.lst}
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