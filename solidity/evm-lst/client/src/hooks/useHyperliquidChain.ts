import { createConfig, http } from "wagmi";
import { defineChain } from "viem";
import { injected } from "wagmi/connectors";

// Define the Hyperliquid Testnet as a custom viem/wagmi chain
export const hyperliquidTestnet = defineChain({
    id: 998, // Placeholder chain ID, check Stablechain docs for latest/correct ID
    name: "Hyperliquid Testnet",
    nativeCurrency: {
        decimals: 18,
        name: "Hype",
        symbol: "Hype",
    },
    rpcUrls: {
        default: {
            http: ["https://rpc.hyperliquid-testnet.xyz/evm"],
        },
    },
    blockExplorers: {
        default: {
            name: "Hyperliquid Explorer",
            url: "https://testnet.purrsec.com/",
        }, // Placeholder/Example
    },
    contracts: {
        // Contract definitions are handled below
    },
});

// Configuration now only uses the injected connector (e.g., MetaMask, Trust Wallet)
export const wagmiConfig = createConfig({
    chains: [hyperliquidTestnet],
    connectors: [injected()],
    transports: {
        [hyperliquidTestnet.id]: http(),
    },
});

// Precompiled Contract Addresses
export const LIQUID_STAKING_CONTRACT_ADDRESS =
    "0x42C38AD2701e04Ad7925A0518a8Ec4F3059d363C" as const;

// Helper constants for the interfaces
import STAKING_ABI from "@/contracts/ILiquidStakingManager.json";

export const STAKING_CONTRACT = {
    address: LIQUID_STAKING_CONTRACT_ADDRESS,
    abi: STAKING_ABI,
    chainId: hyperliquidTestnet.id,
} as const;
