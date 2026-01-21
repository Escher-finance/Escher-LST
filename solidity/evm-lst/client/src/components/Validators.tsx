"use client";

import { useReadContract } from "wagmi";
import { VALIDATOR_CONTRACT } from "@/hooks/useHyperliquidChain";

export function Validators() {
    const {
        data: validators,
        isError,
        isLoading,
    } = useReadContract({
        address: VALIDATOR_CONTRACT.address,
        abi: VALIDATOR_CONTRACT.abi,
        functionName: "getAllValidators",
        args: [],
        chainId: VALIDATOR_CONTRACT.chainId, // Stable testnet chain ID (verify this)
    });

    if (isError) {
        console.error("Error fetching validators:", isError);
    }

    return (
        <div
            style={{
                padding: "5px 5px 0px 5px",
                margin: "0px",
            }}
        >
            <h4>Validators</h4>
            {isLoading ? (
                <p>Loading validators...</p>
            ) : isError ? (
                <p style={{ color: "red" }}>Error loading validators.</p>
            ) : (
                <ul style={{ listStyleType: "none", padding: 0 }}>
                    {validators.map((v, index) => (
                        <li
                            key={index}
                            style={{
                                border: "1px solid #eee",
                                padding: "0px",
                                margin: "0px",
                            }}
                        >
                            <strong>Address:</strong> {v.validator}
                            <br />
                            <strong>Weight:</strong> {v.weight}
                        </li>
                    ))}
                </ul>
            )}
        </div>
    );
}
