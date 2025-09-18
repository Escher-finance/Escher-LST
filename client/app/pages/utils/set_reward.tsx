"use client";

import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
} from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";

export default function SetReward() {
    const { userAddress, client, network } = useGlobalContext();

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
        if (!userAddress) {
            alert("error no user");
            return;
        }
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const reward_contract = formEntries.reward_contract.toString();
        const msg = {
            set_parameters: {
                reward_address: reward_contract,
                cw20_address: "cosmos1t3f4zxve6725sf4glrnlar8uku78j0nyfl0ppzgfju9ft9phvqwqren6rp"
            }
        };

        console.log(JSON.stringify(msg));
        try {
            const res = await client?.execute(userAddress, network?.contracts.lst, msg, "auto");
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
                            name="reward_contract"
                            label="Reward Contract"
                            defaultValue={network?.contracts.reward}
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