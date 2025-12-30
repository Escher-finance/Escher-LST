"use client";

import { ConnectWallet } from "@/components/ConnectWallet";
import { UserBalance } from "@/components/UserBalance";
import { Suspense } from "react";
import { StakingUI } from "@/components/StakingUI";
import { Validators } from "@/components/Validators";

export default function Home() {
    return (
        <div style={{ fontFamily: "Arial, sans-serif", padding: "20px" }}>
            <h1>Escher Staking</h1>
            <div style={{ display: "flex", flex: "2 1" }}>
                <UserBalance />
                <div
                    style={{
                        display: "flex",
                        flexBasis: "30%",
                        flexDirection: "column",
                    }}
                >
                    <Suspense fallback={<p>Loading wallet...</p>}>
                        <ConnectWallet />
                    </Suspense>
                    <Validators />
                </div>
            </div>

            <Suspense fallback={<p>Loading validators...</p>}>
                <StakingUI />
            </Suspense>
        </div>
    );
}
