"use client";

import { useConnection, useWriteContract } from "wagmi";
import { useState, useMemo, useEffect } from "react";
import { parseUnits } from "viem";
import {
    STAKING_CONTRACT,
    STAKING_TOKEN_CONTRACT,
} from "@/hooks/useHyperliquidChain";

export function StakingUI() {
    const [isClient, setIsClient] = useState(false);
    const { address: userAddress, isConnected } = useConnection();
    const [amount, setAmount] = useState("0.0000001");
    const [unbondAmount, setUnbondAmount] = useState("0.0000001");

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

    const unbondAmountBigInt = useMemo(() => {
        try {
            if (unbondAmount) return parseUnits(unbondAmount, 18);
        } catch {
            return BigInt(0);
        }
        return BigInt(0);
    }, [unbondAmount]);

    if (!isClient) {
        return <p>Loading...</p>; // Static fallback for SSR
    }

    const handleBond = (e: React.FormEvent) => {
        e.preventDefault();
        console.log("delegatorAddress", userAddress);
        console.log("amountBigInt", amountBigInt.toString());

        if (!userAddress || amountBigInt === BigInt(0)) {
            console.error("Invalid input: Missing userAddress, or amount.");
            return;
        }

        alert(userAddress + " is delegating " + amount + " hype to validator ");

        writeContract.mutate(
            {
                abi: STAKING_CONTRACT.abi,
                address: STAKING_CONTRACT.address, // Staking contract address
                functionName: "bond",
                args: [amountBigInt, userAddress],
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
    };

    const handleUnbond = async (e: React.FormEvent) => {
        e.preventDefault();
        console.log("delegatorAddress", userAddress);
        console.log("amountBigInt", unbondAmountBigInt.toString());

        if (!userAddress || unbondAmountBigInt === BigInt(0)) {
            console.error("Invalid input: Missing userAddress, or amount.");
            return;
        }

        alert(
            userAddress + " request unbond " + unbondAmountBigInt + " eHype ",
        );

        await writeContract.mutateAsync(
            {
                abi: STAKING_TOKEN_CONTRACT.abi,
                address: STAKING_TOKEN_CONTRACT.address, // Staking contract address
                functionName: "approve",
                args: [STAKING_CONTRACT.address, unbondAmountBigInt],
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

        await writeContract.mutateAsync(
            {
                abi: STAKING_CONTRACT.abi,
                address: STAKING_CONTRACT.address, // Staking contract address
                functionName: "unbondRequest",
                args: [unbondAmountBigInt, userAddress],
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
    };

    const handleSubmitBatch = (e: React.FormEvent) => {
        e.preventDefault();

        writeContract.mutate(
            {
                abi: STAKING_CONTRACT.abi,
                address: STAKING_CONTRACT.address, // Staking contract address
                functionName: "submitBatch",
                args: [],
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
    };

    if (!isConnected) return null;

    return (
        <div
            style={{
                margin: "20px 0",
                padding: "15px",
                border: "1px solid blue",
                display: "flex",
            }}
        >
            <div
                style={{
                    margin: "10px 0",
                    padding: "15px",
                }}
            >
                <h2>💰 Bond</h2>
                <form onSubmit={handleBond}>
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
            <div
                style={{
                    margin: "10px 0",
                    padding: "15px",
                }}
            >
                <h2>💰 Unbond</h2>
                <form onSubmit={handleUnbond}>
                    <div style={{ marginBottom: "10px" }}>
                        <label>
                            Amount (eHype):
                            <input
                                value={unbondAmount}
                                onChange={(e) =>
                                    setUnbondAmount(e.target.value)
                                }
                                required
                                style={{ marginLeft: "10px" }}
                            />
                        </label>
                    </div>

                    <button type="submit">Unbond</button>
                </form>
            </div>

            <div
                style={{
                    margin: "10px 0",
                    padding: "15px",
                }}
            >
                <h2>Submit Batch</h2>
                <form onSubmit={handleSubmitBatch}>
                    <button type="submit">Submit Batch</button>
                </form>
            </div>
        </div>
    );
}
