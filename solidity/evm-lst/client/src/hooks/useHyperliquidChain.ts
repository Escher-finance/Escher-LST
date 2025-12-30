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
    "0x66307cE7d5F1f1880Eb256534e043A7c7AEfF99D" as const;

export const DELEGATION_MANAGER_ADDRESS =
    "0xE77A50c077Abb0c26Ef8fDa2C7474C684cC1Ce6b" as const;

export const LST_ADDRESS =
    "0x6209Be044468f3504A937018A9f97FA2c5d4E44a" as const;

export const LIQUID_STAKING_CONTRACT_ADDRESS =
    "0xE2Ee8Aa4B9624B4605Bf9ad927Ce36D0d95209bb" as const;

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
