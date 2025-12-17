"use client";

import { useConnection, useBalance } from "wagmi";
import { hyperliquidTestnet } from "@/hooks/useHyperliquidChain";
import { useEffect, useState } from "react";

export function UserBalance() {
    const [isClient, setIsClient] = useState(false);
    const [showBalance, setShowBalance] = useState(false);
    const { address, status } = useConnection();
    const { data, isLoading, isError } = useBalance({
        address,
        chainId: hyperliquidTestnet.id,
    });

    useEffect(() => {
        setIsClient(true);

        if (status !== "connected") {
            setShowBalance(true);
        }
    }, []);

    useEffect(() => {
        setShowBalance(true);
        console.log(data);
    }, [isLoading]);

    if (!isClient) {
        return <p>Loading...</p>; // Static fallback for SSR
    }

    return (
        <div
            style={{
                padding: "10px",
                border: "1px solid #ccc",
                margin: "10px 0",
            }}
        >
            {showBalance && (
                <p>
                    **Wallet Balance**:{" "}
                    {data ? `${data.value} ${data.symbol}` : "N/A"}
                </p>
            )}
        </div>
    );
}
