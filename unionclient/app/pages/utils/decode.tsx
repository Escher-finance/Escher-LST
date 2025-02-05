"use client";


import { useState } from "react";
import {
    Button,
    Textarea,
} from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";
import { MsgTransfer } from "cosmjs-types/ibc/applications/transfer/v1/tx";

export default function DecodePacket() {
    const { userAddress, client, network } = useGlobalContext();
    const [data, setData] = useState("");

    const handleSubmit = async (e: any) => {
        // Prevent the browser from reloading the page
        e.preventDefault();
        const form = e.target;
        const formData = new FormData(form);
        const formEntries = Object.fromEntries(formData.entries());
        const packet_data = formEntries.packet_data.toString();

        try {
            const packet_bytes =  Uint8Array.from(Buffer.from(packet_data, 'hex'));
            console.log(packet_bytes);

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
                            name="packet_data"
                            labelPlacement="outside"
                            placeholder="Enter your packet data"
                            value="000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001e000000000000000000000000000000000000000000000000000000000000000144aaa51a0814d91f7d2b3ab60829a921ec9eb8e170000000000000000000000000000000000000000000000000000000000000000000000000000000000000020f91dfdde22bdcd4c63d2b38759dbe23bd4f9b3f8977467b0826d2af797dd4c8300000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000003e800000000000000000000000000000000000000000000000000000000000000393078366632373036303866623536323133333737376166306637316636333836666663313733376333302f6368616e6e656c2d332f6d756e6f000000000000000000000000000000000000000000000000000000000000000000000000000000"
                            onValueChange={setData}
                            minRows={5}

                        />
                    </div>
                        <Button type="submit">Query</Button>
            </form>
        </div>
    );
}
