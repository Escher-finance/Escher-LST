"use client";

import { useConnection, useWriteContract, useReadContract } from "wagmi";
import { useState, useMemo, useEffect } from "react";
import { parseUnits } from "viem";
import {
    STAKING_CONTRACT,
    STAKING_TOKEN_CONTRACT,
} from "@/hooks/useHyperliquidChain";

export function StakingUI() {
    const [isClient, setIsClient] = useState(false);
    const { address: userAddress, isConnected } = useConnection();
    const [amount, setAmount] = useState("0.000001");
    const [unbondAmount, setUnbondAmount] = useState("0.000001");

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

    const {
        data: userRequests,
        isError: userRequestsError,
        isLoading: userRequestsLoading,
    } = useReadContract({
        address: STAKING_CONTRACT.address,
        abi: STAKING_CONTRACT.abi,
        functionName: "getUserRequestIds",
        args: [userAddress],
        chainId: STAKING_CONTRACT.chainId,
    });

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

    const handleReceiveBatch = (e: React.FormEvent) => {
        e.preventDefault();

        writeContract.mutate(
            {
                abi: STAKING_CONTRACT.abi,
                address: STAKING_CONTRACT.address, // Staking contract address
                functionName: "receiveBatch",
                args: [1],
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

    const handleClaim = (e: React.FormEvent) => {
        e.preventDefault();
        const id = e.target.elements.requestId.value;

        writeContract.mutate(
            {
                abi: STAKING_CONTRACT.abi,
                address: STAKING_CONTRACT.address, // Staking contract address
                functionName: "claimUnbondRequest",
                args: [BigInt(id)],
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

    const handleMoveBatch = (e: React.FormEvent) => {
        e.preventDefault();

        writeContract.mutate(
            {
                abi: STAKING_CONTRACT.abi,
                address: STAKING_CONTRACT.address, // Staking contract address
                functionName: "moveBatch",
                args: [1],
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
                margin: "5px",
                padding: "5px",
                border: "1px solid blue",
                display: "flex",
            }}
        >
            <div
                style={{
                    margin: "5px 0",
                    padding: "5px",
                }}
            >
                <h4>💰 Bond</h4>
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
                    margin: "5px 0",
                    padding: "5px",
                }}
            >
                <h4>💰 Unbond</h4>
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
                    display: "flex",
                    flexDirection: "column",
                    margin: "10px 0",
                    padding: "15px",
                }}
            >
                <div
                    style={{
                        margin: "5px",
                        padding: "5px",
                    }}
                >
                    <form onSubmit={handleSubmitBatch}>
                        <button type="submit">Submit Batch</button>
                    </form>
                </div>
                <div
                    style={{
                        margin: "5px",
                        padding: "5px",
                    }}
                >
                    <form onSubmit={handleMoveBatch}>
                        <button type="submit">Move Batch</button>
                    </form>
                </div>
                <div
                    style={{
                        margin: "5px",
                        padding: "5px",
                    }}
                >
                    <form onSubmit={handleReceiveBatch}>
                        <button type="submit">Receive Batch</button>
                    </form>
                </div>
            </div>
            {Array.isArray(userRequests) && userRequests.length > 0 && (
                <div>
                    <h4>Claim Unbond:</h4>
                    <form onSubmit={handleClaim}>
                        {userRequests ? (
                            <select id="requestId" name="requestId">
                                {Array.isArray(userRequests) &&
                                    userRequests.map((v, index) => (
                                        <option key={index} value={v}>
                                            {" "}
                                            Request Id: {v}{" "}
                                        </option>
                                    ))}
                            </select>
                        ) : (
                            ""
                        )}
                        <button type="submit">Claim</button>
                    </form>
                </div>
            )}
        </div>
    );
}
