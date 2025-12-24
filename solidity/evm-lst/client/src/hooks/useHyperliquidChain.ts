import { createConfig, http } from "wagmi";
import { defineChain } from "viem";
import { injected } from "wagmi/connectors";
import { erc20Abi } from "viem";
import delegationABI from "@/contracts/IDelegationManager.json";
import validatorABI from "@/contracts/IValidatorSetManager.json";
import l1ReadABI from "@/contracts/L1Read.json";

// Define the Hyperliquid Testnet as a custom viem/wagmi chain
export const hyperliquidTestnet = defineChain({
    id: 998,
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

export const VALIDATOR_MANAGER_ADDRESS =
    "0xba14B5328709Ca3af894EE9DE26B0BD0DA3150DF" as const;

export const DELEGATION_MANAGER_ADDRESS =
    "0x84Fc6e5c928F43aBd61682900Ca7bCC67b7cDBFA" as const;

export const LST_ADDRESS =
    "0x9639348F84A2E78A4Fe8eC2085948BdEEb45FE98" as const;

export const LIQUID_STAKING_CONTRACT_ADDRESS =
    "0x8cabC88c664D319Da1Ca390027fdFd628650C50e" as const;

export const L1READ_ADDRESS =
    "0xb0FBCF71E600383C1298413BFDEFc4A32240B033" as const;

// Helper constants for the interfaces
import STAKING_ABI from "@/contracts/ILiquidStakingManager.json";

export const STAKING_CONTRACT = {
    address: LIQUID_STAKING_CONTRACT_ADDRESS,
    abi: STAKING_ABI,
    chainId: hyperliquidTestnet.id,
} as const;

export const STAKING_TOKEN_CONTRACT = {
    address: LST_ADDRESS,
    abi: erc20Abi,
    chainId: hyperliquidTestnet.id,
} as const;

export const DELEGATION_CONTRACT = {
    address: DELEGATION_MANAGER_ADDRESS,
    abi: delegationABI,
    chainId: hyperliquidTestnet.id,
} as const;

export const VALIDATOR_CONTRACT = {
    address: VALIDATOR_MANAGER_ADDRESS,
    abi: validatorABI,
    chainId: hyperliquidTestnet.id,
} as const;

export const L1READ_CONTRACT = {
    address: L1READ_ADDRESS,
    abi: l1ReadABI,
    chainId: hyperliquidTestnet.id,
} as const;
