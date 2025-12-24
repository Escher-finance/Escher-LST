"use client";

import { useConnection, useConnect, useDisconnect } from "wagmi";
import { injected } from "wagmi/connectors";
import { useEffect, useState } from "react";

export function ConnectWallet() {
    const [isClient, setIsClient] = useState(false);
    const { address, isConnected } = useConnection();
    const connect = useConnect();
    const disconnect = useDisconnect();

    useEffect(() => {
        setIsClient(true);
    }, []);

    if (!isClient) {
        return <p>Loading...</p>; // Static fallback for SSR
    }

    if (isConnected) {
        return (
            <div
                style={{
                    padding: "10px",
                    border: "1px solid green",
                    margin: "10px 0",
                    flexBasis: "30%",
                }}
            >
                <p>Connected to: **{address}**</p>
                <button onClick={() => disconnect.mutate()}>Disconnect</button>
            </div>
        );
    }

    return (
        <button onClick={() => connect.mutate({ connector: injected() })}>
            Connect Wallet
        </button>
    );
}
