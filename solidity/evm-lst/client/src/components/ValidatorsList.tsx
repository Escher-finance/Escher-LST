"use client";

import { STAKING_CONTRACT } from "@/hooks/useStablechain";
import { useReadContract } from "wagmi";
import { useAccount, useBalance } from "wagmi";
import { stableTestnet } from "@/hooks/useStablechain";
import { useEffect, useState } from "react";
import contractABI from "@/contracts/staking.json";
import { formatUnits } from "viem";
import { useMemo, Suspense } from "react";

type Validator = {
    operatorAddress: string;
    consensusPubkey: string;
    jailed: boolean;
    status: string;
    tokens: string;
    delegatorShares: string;
    description: string;
    unbondingHeight: string;
    unbondingTime: string;
    commission: string;
    minSelfDelegation: string;
};

function getStatusString(status: number): string {
    const statuses: Record<number, string> = {
        0: "BOND_STATUS_UNSPECIFIED",
        1: "BOND_STATUS_UNBONDING",
        2: "BOND_STATUS_UNBONDED",
        3: "BOND_STATUS_BONDED",
    };
    return statuses[status] || "UNKNOWN";
}

export function ValidatorsList() {
    const { data, isError, isLoading } = useReadContract({
        address: STAKING_CONTRACT.address,
        abi: contractABI,
        functionName: "validators",
        args: [
            "BOND_STATUS_BONDED",
            {
                key: "0x",
                offset: BigInt(0),
                limit: BigInt(100),
                countTotal: false,
                reverse: false,
            },
        ],
        chainId: 2201, // Stable testnet chain ID (verify this)
    });

    const validators: Validator[] = useMemo(() => {
        if (!data || !Array.isArray(data)) return [];
        const [validatorData] = data;
        return validatorData.map((v: any) => ({
            operatorAddress: v.operatorAddress,
            consensusPubkey: v.consensusPubkey,
            jailed: v.jailed,
            status: getStatusString(v.status),
            tokens: formatUnits(v.tokens, 18),
            delegatorShares: formatUnits(v.delegatorShares, 18),
            description: v.description,
            unbondingHeight: v.unbondingHeight.toString(),
            unbondingTime: new Date(
                Number(v.unbondingTime) * 1000,
            ).toLocaleString(),
            commission: formatUnits(v.commission, 18),
            minSelfDelegation: formatUnits(v.minSelfDelegation, 18),
        }));
    }, [data]);

    if (isError) {
        console.error("Error fetching validators:", isError);
    }

    return (
        <div>
            <h2>Validators</h2>
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
                                padding: "10px",
                                margin: "5px 0",
                            }}
                        >
                            <strong>Moniker:</strong> {v.description}
                            <br />
                            <strong>Address:</strong> {v.operatorAddress}
                            <br />
                            <strong>Tokens:</strong> {v.tokens}
                            <br />
                            <strong>Status:</strong> {v.status}
                            <br />
                            <strong>Commission:</strong> {v.commission}
                            <br />
                            <strong>Min Self Delegation:</strong>{" "}
                            {v.minSelfDelegation}
                            <br />
                            <strong>Unbonding Time:</strong> {v.unbondingTime}
                        </li>
                    ))}
                </ul>
            )}
        </div>
    );
}
