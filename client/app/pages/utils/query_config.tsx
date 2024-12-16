"use client";


import { useState } from "react";
import {
    Card,
    CardBody,
    CardFooter,
    Button,
    Input,
    Textarea,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";

export default function QueryConfig() {
    const { userAddress, client, network } = useGlobalContext();
    const [totalBond, setTotalBond] = useState("");
    const [data, setData] = useState("");

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
   
        try {
            const msg = {
                validators: {}
            };
            const lst_contract = "union1tkqqlr3xdvr20ywnmtjgrdstqdepj74teq5vmltgl2zvsn9sgwusrdfv8y"
            const res = await client.queryContractSmart(
                lst_contract,
                msg
            );
            let data = "Validators: " + JSON.stringify(res.validators);
            

            const res2 = await client.queryContractSmart(
                lst_contract,
                { balance :{}}
            );

            data +=  "\nBalance " + JSON.stringify(res2);

            const res3 = await client.queryContractSmart(
                lst_contract,
                { log :{}}
            );

            data += "\nLog " + JSON.stringify(res3);

            const res4 = await client.queryContractSmart(
                lst_contract,
                { state :{}}
            );

            data += "\nState : " + JSON.stringify(res4);

            setData(data);
        } catch (err) {
            console.log(err);
        }
    };

    return (
        <div className="w-full">
            <form onSubmit={handleSubmit}>
                    <div className="text-left">
                        Validators:
                        <Textarea
                            variant="underlined"
                            labelPlacement="outside"
                            placeholder="Enter your description"
                            value={data}
                            onValueChange={setData}
                            minRows={5}

                        />
                    </div>
                        <Button type="submit">Query</Button>
            </form>
        </div>
    );
}
