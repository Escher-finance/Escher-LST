"use client";

import {
    DELEGATION_CONTRACT,
    STAKING_CONTRACT,
    STAKING_TOKEN_CONTRACT,
    VALIDATOR_CONTRACT,
} from "@/hooks/useHyperliquidChain";

export function Contracts() {
    return (
        <div
            style={{
                padding: "0px 5px 5px 10px",
                margin: "0px",
            }}
        >
            <h4>Contracts</h4>
            <strong>Lst (eHype)</strong>:{" "}
            <a
                href={
                    "https://testnet.purrsec.com/address/" +
                    STAKING_TOKEN_CONTRACT.address
                }
                target="_blank"
            >
                {STAKING_TOKEN_CONTRACT.address}
            </a>
            <br />
            <strong>Lst Manager</strong>:{" "}
            <a
                href={
                    "https://testnet.purrsec.com/address/" +
                    STAKING_CONTRACT.address
                }
                target="_blank"
            >
                {STAKING_CONTRACT.address}
            </a>
            <br />
            <strong>Delegation Manager</strong>:{" "}
            <a
                href={
                    "https://testnet.purrsec.com/address/" +
                    DELEGATION_CONTRACT.address
                }
                target="_blank"
            >
                {DELEGATION_CONTRACT.address}
            </a>
            <br />
            <strong>Validator Manager</strong>:{" "}
            <a
                href={
                    "https://testnet.purrsec.com/address/" +
                    VALIDATOR_CONTRACT.address
                }
                target="_blank"
            >
                {VALIDATOR_CONTRACT.address}
            </a>
        </div>
    );
}
