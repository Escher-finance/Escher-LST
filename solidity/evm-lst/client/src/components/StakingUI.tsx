"use client";

import {
    useConnection,
    useWriteContract,
    useWaitForTransactionReceipt,
} from "wagmi";
import { useState, useMemo, useEffect } from "react";
import { parseUnits } from "viem";
import { STAKING_CONTRACT } from "@/hooks/useHyperliquidChain";

export function StakingUI() {
    const [isClient, setIsClient] = useState(false);
    const { address: delegatorAddress, isConnected } = useConnection();
    const [amount, setAmount] = useState("0.0000001");

    const writeContract = useWriteContract();

    useEffect(() => {
        setIsClient(true);
    }, []);

    const amountBigInt = useMemo(() => {
        try {
            if (amount) return parseUnits(amount, 18);
        } catch {
            return BigInt(0);
        }
        return BigInt(0);
    }, [amount]);

    if (!isClient) {
        return <p>Loading...</p>; // Static fallback for SSR
    }

    const handleDelegate = (e: React.FormEvent) => {
        e.preventDefault();
        console.log("delegatorAddress", delegatorAddress);
        console.log("amountBigInt", amountBigInt.toString());

        if (!delegatorAddress || amountBigInt === BigInt(0)) {
            console.error(
                "Invalid input: Missing delegatorAddress, validatorAddress, or amount.",
            );
            return;
        }

        alert(
            delegatorAddress +
                " is delegating " +
                amount +
                " hype to validator ",
        );

        try {
            writeContract.mutate(
                {
                    abi: STAKING_CONTRACT.abi,
                    address: STAKING_CONTRACT.address, // Staking contract address
                    functionName: "delegate",
                    args: [amountBigInt],
                    value: amountBigInt,
                },
                {
                    onSuccess: (data) => {
                        console.log("Transaction successful:", data);
                        alert(`Transaction successful! Hash: ${data}`); // Updated to use data directly
                    },
                    onError: (error) => {
                        console.error("Transaction failed:", error);
                        alert(`Transaction failed: ${error.message}`);
                    },
                },
            );
            console.log("writeContract called successfully.");
        } catch (error) {
            console.error("Error calling writeContract:", error);
        }
    };

    if (!isConnected) return null;

    return (
        <div
            style={{
                margin: "20px 0",
                padding: "15px",
                border: "1px solid blue",
            }}
        >
            <h2>💰 Delegate / Stake</h2>

            <form onSubmit={handleDelegate}>
                <div style={{ marginBottom: "10px" }}>
                    <label>
                        Amount (Hype):
                        <input
                            value={amount}
                            onChange={(e) => setAmount(e.target.value)}
                            required
                            style={{ marginLeft: "10px" }}
                        />
                    </label>
                </div>

                <button type="submit">Delegate</button>
            </form>
        </div>
    );
}
