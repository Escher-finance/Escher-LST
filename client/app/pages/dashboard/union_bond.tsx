

"use client";

import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
} from "@heroui/react";
import { useGlobalContext } from "@/app/core/context";
import { useState } from "react";
interface KeyProps {
    stateKey: number;
    setStateKey: (key: number) => void;
}

export default function UnionBond({ stateKey, setStateKey }: KeyProps) {
    const { userAddress, client, network } = useGlobalContext();

    const [isLoading, setIsLoading] = useState(false);

    const handleSubmit = async (e: any) => {

        e.preventDefault();
        if (!userAddress) {
            alert("no user");
            return;
        }


        const liquidity = await client?.queryContractSmart(
            network?.contracts.lst,
            {
                accounting_state: {}
            }
        );

        const rate = liquidity.purchase_rate;

        console.log("rate: ", rate);


        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const amount = formEntries.amount.toString();


        const msg = {
            bond: {
                mint_to_address: userAddress,
                min_mint_amount: Math.floor(Number(amount) * Number(rate)).toString(),
            },
        };

        if (Number(amount) < 1000) {
            alert("Sorry, minimal bond amount is 1000000");
            return;
        }

        console.log(JSON.stringify(msg));

        try {
            setIsLoading(true);
            const funds = [{
                amount,
                denom: network?.stakeCurrency.coinMinimalDenom
            }];
            const res = await client?.execute(userAddress, network?.contracts.lst, msg, "auto", "execute bond", funds);
            alert(res?.transactionHash);
            let newKey = stateKey + 1;
            setStateKey(newKey);
            setIsLoading(false);
        } catch (err) {
            console.log(err);
            setIsLoading(false);
        }
    };

    return (
        <div className="w-full flex flex-row gap-4">
            <form onSubmit={handleSubmit} className="w-full flex">
                <Card className="w-full flex">
                    <CardBody className="gap-4">
                        <Input
                            isRequired
                            name="amount"
                            label="Amount"
                            defaultValue="1000000000000000000"
                        />
                    </CardBody>
                    <CardFooter>
                        <Button type="submit" isLoading={isLoading}>Bond</Button>
                    </CardFooter>
                </Card>
            </form>
        </div>
    );
}

//bbn1fh0yyvuxz7l0vcusq5jc9zvzpm8ec2auvvkh44
//xion1vnglhewf3w66cquy6hr7urjv3589srhe496gds