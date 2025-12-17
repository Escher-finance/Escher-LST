import { http, createConfig } from 'wagmi'
import { mainnet } from 'wagmi/chains'
import { getDefaultConfig } from 'connectkit'

export const config = createConfig(
    getDefaultConfig({
        chains: [
            {
                ...mainnet,
                id: 1337, // Replace with StableChain's chainId if known
                name: 'StableChain Testnet',
                rpcUrls: {
                    default: {
                        http: ['https://rpc.testnet.stable.xyz'],
                    },
                },
                nativeCurrency: {
                    name: 'STC',
                    symbol: 'STC',
                    decimals: 18,
                },
                blockExplorers: {
                    default: { name: 'StableScan', url: 'https://explorer.stable.xyz' },
                },
                testnet: true,
            },
        ],
        walletConnectProjectId: 'YOUR_WALLETCONNECT_PROJECT_ID', // Replace with your WalletConnect Project ID
        appName: 'StableChain Staking',
    })
)
