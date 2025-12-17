'use client';

import { useAccount, useReadContract } from 'wagmi';
import { DISTRIBUTION_CONTRACT, STAKING_CONTRACT } from '@/hooks/useStablechain';
import { formatUnits } from 'viem';

// Simplified types for the rewards/delegation structures
type DecCoin = {
    denom: string;
    amount: bigint;
    precision: number;
};

// Sub-component to fetch and display details for a single delegation
function DelegationDetails({ delegatorAddress, validatorAddress }: { delegatorAddress: `0x${string}`, validatorAddress: `0x${string}` }) {

    // 1. Get Staked Amount (Shares/Balance) from IStableStaking
    const { data: delegationData, isLoading: isLoadingDelegation } = useReadContract({
        ...STAKING_CONTRACT,
        functionName: 'delegation',
        args: [delegatorAddress, validatorAddress],
        query: {
            staleTime: 5000,
        }
    });

    // The 'delegation' function returns (shares, balance). We want the balance (Coin struct at index 1) amount.
    const stakedBalance = (delegationData as any)?.[1]?.amount;

    // 2. Get Rewards from IStableDistribution using delegationRewards
    const { data: rewardsData, isLoading: isLoadingRewards } = useReadContract({
        ...DISTRIBUTION_CONTRACT,
        functionName: 'delegationRewards',
        args: [delegatorAddress, validatorAddress],
        query: {
            staleTime: 5000,
        }
    });

    const rewards = rewardsData as DecCoin[] | undefined;

    const isLoading = isLoadingDelegation || isLoadingRewards;

    return (
        <li style={{ border: '1px solid #eee', padding: '10px', margin: '5px 0' }}>
            **Validator**: {validatorAddress.substring(0, 15)}...
            <br />
            {isLoading ? (
                <p>Loading details...</p>
            ) : (
                <>
                    **Staked Amount (IStableStaking)**: {stakedBalance ? formatUnits(stakedBalance, 18) : '0'} STABLE
                    <br />
                    **Pending Rewards (IStableDistribution)**:
                    {rewards && rewards.length > 0 ? (
                        rewards.map((r, i) => (
                            <span key={i} style={{ marginLeft: '5px' }}>
                                {formatUnits(r.amount, 18)} {r.denom}
                            </span>
                        ))
                    ) : (
                        <span> 0</span>
                    )}
                </>
            )}
        </li>
    );
}

export function DelegationList() {
    const { address: delegatorAddress, isConnected } = useAccount();

    // Get the list of validators the user has delegated to (via Distribution module, 
    // which is the common pattern in Cosmos/EVM chains for this list)
    const { data: delegatedValidatorsData, isLoading: isLoadingValidatorList } = useReadContract({
        ...DISTRIBUTION_CONTRACT,
        functionName: 'delegatorValidators',
        args: [delegatorAddress!],
        query: {
            enabled: isConnected && !!delegatorAddress,
            staleTime: 60000,
        },
    });

    // The delegatorValidators returns string addresses, convert them to hex addresses
    const validatorAddresses = (delegatedValidatorsData as string[] | undefined)?.map(addr => `0x${addr.slice(2)}` as `0x${string}`);

    if (!isConnected || !delegatorAddress) return null;

    return (
        <div style={{ margin: '20px 0', padding: '15px', border: '1px solid purple' }}>
            <h2>🔗 My Current Delegations & Rewards</h2>

            {isLoadingValidatorList ? (
                <p>Loading delegated validator list...</p>
            ) : validatorAddresses && validatorAddresses.length > 0 ? (
                <ul style={{ listStyleType: 'none', padding: 0 }}>
                    {validatorAddresses.map((vAddr) => (
                        <DelegationDetails
                            key={vAddr}
                            delegatorAddress={delegatorAddress}
                            validatorAddress={vAddr}
                        />
                    ))}
                </ul>
            ) : (
                <p>You have no active delegations yet.</p>
            )}
        </div>
    );
}