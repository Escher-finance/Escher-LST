import { createConfig, http } from 'wagmi';
import { defineChain } from 'viem';
import { injected } from 'wagmi/connectors';

// Define the STablechain Testnet as a custom viem/wagmi chain
export const stableTestnet = defineChain({
    id: 2201, // Placeholder chain ID, check Stablechain docs for latest/correct ID
    name: 'Stablechain Testnet',
    nativeCurrency: {
        decimals: 18,
        name: 'gUSDT',
        symbol: 'gUSDT',
    },
    rpcUrls: {
        default: {
            http: ['https://rpc.testnet.stable.xyz'],
        },
    },
    blockExplorers: {
        default: { name: 'Stable Explorer', url: 'https://explorer.testnet.stable.xyz' }, // Placeholder/Example
    },
    contracts: {
        // Contract definitions are handled below
    }
});

// Configuration now only uses the injected connector (e.g., MetaMask, Trust Wallet)
export const wagmiConfig = createConfig({
    chains: [stableTestnet],
    connectors: [
        injected(),
    ],
    transports: {
        [stableTestnet.id]: http(),
    },
});

// Precompiled Contract Addresses
export const STAKING_CONTRACT_ADDRESS = '0x0000000000000000000000000000000000000800' as const;
export const DISTRIBUTION_CONTRACT_ADDRESS = '0x0000000000000000000000000000000000000801' as const;

// Helper constants for the interfaces
import STAKING_ABI from '@/contracts/staking.json';
import DISTRIBUTION_ABI from '@/contracts/IStableDistribution.json';

export const STAKING_CONTRACT = {
    address: STAKING_CONTRACT_ADDRESS,
    abi: STAKING_ABI,
    chainId: stableTestnet.id,
} as const;

export const DISTRIBUTION_CONTRACT = {
    address: DISTRIBUTION_CONTRACT_ADDRESS,
    abi: DISTRIBUTION_ABI,
    chainId: stableTestnet.id,
} as const;