import { http, createConfig } from 'wagmi'
import { mainnet } from 'wagmi/chains'
import { getDefaultConfig } from 'connectkit'

export const config = createConfig(
    getDefaultConfig({
        chains: [
            {
                id: 998, // Replace with StableChain's chainId if known
                name: 'Hyperliquid Testnet',
                rpcUrls: {
                    default: {
                        http: ['https://rpc.hyperliquid-testnet.xyz/evm'],
                    },
                },
                nativeCurrency: {
                    name: 'Hype',
                    symbol: 'HYPE',
                    decimals: 18,
                },
                blockExplorers: {
                    default: { name: 'StableScan', url: 'https://testnet.purrsec.com' },
                },
                testnet: true,
            },
        ],
        walletConnectProjectId: 'YOUR_WALLETCONNECT_PROJECT_ID', // Replace with your WalletConnect Project ID
        appName: 'StableChain Staking',
    })
)
