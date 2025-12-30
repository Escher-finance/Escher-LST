"use client";

import { useConnection, useBalance, useReadContract } from "wagmi";

import {
    DELEGATION_CONTRACT,
    L1READ_CONTRACT,
    STAKING_TOKEN_CONTRACT,
    VALIDATOR_CONTRACT,
} from "@/hooks/useHyperliquidChain";
import { useEffect, useState } from "react";
import { STAKING_CONTRACT } from "@/hooks/useHyperliquidChain";
import { getBalance } from "wagmi/actions";

export function UserBalance() {
    const [isClient, setIsClient] = useState(false);
    const [showBalance, setShowBalance] = useState(false);
    const { address, status } = useConnection();
    const { data, isLoading, isError, error } = useBalance({
        address,
    });

    const {
        data: lstContractData,
        isLoading: lstContractIsLoading,
        isError: lstContractIsError,
        error: lstContractError,
    } = useBalance({
        address: STAKING_CONTRACT.address,
    });

    const {
        data: delContractData,
        isLoading: delContractIsLoading,
        isError: delContractIsError,
        error: delContractError,
    } = useBalance({
        address: DELEGATION_CONTRACT.address,
    });

    const EVM_DECIMALS: number = 1e18;
    const HYPE_TOKEN_INDEX: number = 1105;

    const {
        data: tokenData,
        isError: tokenError,
        isLoading: tokenLoading,
    } = useReadContract({
        address: STAKING_TOKEN_CONTRACT.address,
        abi: STAKING_TOKEN_CONTRACT.abi,
        functionName: "balanceOf",
        args: [
            address ? address : `0x0x15Ee7c367F4232241028c36E720803100757c6e9`,
        ],
        chainId: 998, // Stable testnet chain ID (verify this)
    });

    const {
        data: summaryData,
        isError: summaryDataError,
        isLoading: summaryDataLoading,
    } = useReadContract({
        address: DELEGATION_CONTRACT.address,
        abi: DELEGATION_CONTRACT.abi,
        functionName: "delegationSummary",
        args: [],
        chainId: 998, // Stable testnet chain ID (verify this)
    });

    const {
        data: liquidity,
        isError: liquidityError,
        isLoading: liquidityLoading,
    } = useReadContract({
        address: STAKING_CONTRACT.address,
        abi: STAKING_CONTRACT.abi,
        functionName: "getLiquidity",
        args: [],
        chainId: 998, // Stable testnet chain ID (verify this)
    });

    const {
        data: currentBatch,
        isError: currentBatchError,
        isLoading: currentBatchLoading,
    } = useReadContract({
        address: STAKING_CONTRACT.address,
        abi: STAKING_CONTRACT.abi,
        functionName: "getCurrentBatchId",
        args: [],
        chainId: 998, // Stable testnet chain ID (verify this)
    });

    const {
        data: batch,
        isError: batchError,
        isLoading: batchLoading,
    } = useReadContract({
        address: STAKING_CONTRACT.address,
        abi: STAKING_CONTRACT.abi,
        functionName: "getBatch",
        args: [1],
        chainId: 998, // Stable testnet chain ID (verify this)
    });

    const {
        data: rate,
        isError: rateError,
        isLoading: rateLoading,
    } = useReadContract({
        address: STAKING_CONTRACT.address,
        abi: STAKING_CONTRACT.abi,
        functionName: "rate",
        args: [],
        chainId: 998, // Stable testnet chain ID (verify this)
    });

    const {
        data: spotBalance,
        isError: spotBalanceError,
        isLoading: spotBalanceLoading,
    } = useReadContract({
        address: L1READ_CONTRACT.address,
        abi: L1READ_CONTRACT.abi,
        functionName: "spotBalance",
        args: [DELEGATION_CONTRACT.address, HYPE_TOKEN_INDEX],
        chainId: 998, // Stable testnet chain ID (verify this)
    });

    useEffect(() => {
        setIsClient(true);

        if (status !== "connected") {
            console.warn("Wallet not connected. Status:", status);
            setShowBalance(false);
        } else {
            console.log("Wallet connected. Address:", address);
            setShowBalance(true);
        }
    }, [status, address]);

    useEffect(() => {
        setShowBalance(true);
        console.log("Balance loading status:", isLoading);
        console.log("Balance data:", data);
        if (isError) {
            console.error("Error fetching balance:", isError);
        }
    }, [isLoading, data, isError]);

    useEffect(() => {
        console.log("batchLoading status:", batchLoading);
        console.log("batchError status:", batchError);
    }, [batchLoading, batchError]);

    if (!isClient) {
        return <p>Loading...</p>; // Static fallback for SSR
    }

    return (
        <div
            style={{
                display: "flex",
                flexBasis: "70%",
                padding: "10px",
                border: "1px solid #ccc",
                margin: "10px 0",
            }}
        >
            {showBalance && (
                <div
                    style={{
                        padding: "10px",
                        margin: "10px",
                    }}
                >
                    <strong>EVM User Balance</strong>: <br />
                    <strong>Hype: </strong>
                    {data ? `${data.value} ${data.symbol}` : "0"}
                    <br />
                    <strong>Lst: </strong>
                    {tokenData ? `${tokenData} eHype` : "0"}
                    <br />
                    <br />
                    <strong>LSTContract </strong>
                    <br />
                    Balance:{" "}
                    {lstContractData
                        ? `${lstContractData.value} ${lstContractData.symbol}`
                        : "0"}
                    <br />
                    <br />
                    <strong>DelegationContract </strong>
                    <br />
                    EVM Balance:{" "}
                    {delContractData
                        ? `${delContractData.value} ${delContractData.symbol}`
                        : "0"}
                    <br />
                    Spot Balance:{" "}
                    {spotBalance ? `${spotBalance.total} Hype` : "0"}
                    <br />
                    <br />
                    <strong>Liquidity: </strong>
                    <br />
                    LST(eHype):
                    {liquidity ? `${liquidity.totalLst}` : "0"} eHype
                    <br />
                    Delegated:
                    {summaryData ? `${summaryData?.delegated} Hype` : "0"}
                    <br />
                    Undelegated:{" "}
                    {summaryData ? `${summaryData?.undelegated} Hype` : "0"}
                    <br />
                    TotalPendingWithdrawal:{" "}
                    {summaryData
                        ? `${summaryData?.totalPendingWithdrawal} Hype`
                        : "0"}
                    <br />
                    PendingWithdrawals:{" "}
                    {summaryData ? `${summaryData?.nPendingWithdrawals}` : "0"}
                    <br />
                </div>
            )}

            <div
                style={{
                    padding: "0px",
                    margin: "0px",
                }}
            >
                <div
                    style={{
                        padding: "10px",
                        margin: "10px",
                    }}
                >
                    <strong>Batch</strong>
                    <br />
                    Batch ID: {batch ? `${batch.batchId}` : "0"}
                    <br />
                    Total Shares: {batch ? `${batch.totalShares}` : "0"}
                    <br />
                    Total Assets: {batch ? `${batch.totalAssets}` : "0"}
                    <br />
                    Request Ids: {batch ? `${batch.requestIds}` : "-"}
                    <br />
                    Next Action time: {batch ? `${batch.nextActionTime}` : "-"}
                    <br />
                    Status: {batch ? `${batch.status}` : "-"}
                </div>
                <div
                    style={{
                        padding: "10px",
                        margin: "10px",
                    }}
                >
                    <strong>Rate</strong>
                    <br />
                    Bond Rate:{" "}
                    {rate ? `${Number(rate.bondRate) / EVM_DECIMALS}` : "-"}
                    <br />
                    Unbond Rate:{" "}
                    {rate ? `${Number(rate.unbondRate) / EVM_DECIMALS}` : "-"}
                </div>
            </div>
        </div>
    );
}
