export type ChainConfig = {
    chainId: string;
    chainName: string;
    coinDenom: string,
    lstCoinDenom: string,
    lstCoinSymbol: string,
    rpc: string;
    rest: string;
    ucs03Channel: string;
    ucs03RelayContract: string;
    feeRate: string;
    revenueReceiver: string;
    validator: string;
    codeId: {
        lst: number | undefined;
        cw20: number | undefined;
        reward: number | undefined;
    };
};

export type SupportedNetworks = "uniontestnet" | "lst-network";


export const Networks: Record<SupportedNetworks, ChainConfig> = {
    "lst-network": {
        chainId: "lst",
        chainName: "LST Network",
        rpc: "https://lst.rickyanto.com",
        rest: "https://lst.rickyanto.com",
        coinDenom: "stake",
        lstCoinDenom: "lqstake",
        lstCoinSymbol: "LQSTAKE",
        ucs03Channel: "channel-86",
        ucs03RelayContract: "union1m87a5scxnnk83wfwapxlufzm58qe2v65985exff70z95a2yr86yq7hl08h",
        feeRate: "0.1",
        revenueReceiver: "cosmos1pss37x3hwq5ytk7uhf9fjcpcd7k20pekq6xtlz",
        validator: "cosmosvaloper1h492ust5a9qzhh4zhhhlyva9v8ftn5sz99k4yp",
        codeId: {
            lst: 21,
            cw20: 20,
            reward: 22,
        },
    },
    "uniontestnet": {
        chainId: "union-testnet-9",
        chainName: "uniontestnet",
        rpc: "https://rpc.testnet-9.union.build:443",
        rest: "https://api.testnet-9.union.build:443",
        coinDenom: "muno",
        lstCoinDenom: "limuno",
        lstCoinSymbol: "LIMUNO",
        ucs03Channel: "",
        ucs03RelayContract: "",
        feeRate: "0.1",
        revenueReceiver: "cosmos1pss37x3hwq5ytk7uhf9fjcpcd7k20pekq6xtlz",
        validator: "cosmosvaloper1h492ust5a9qzhh4zhhhlyva9v8ftn5sz99k4yp",
        codeId: {
            lst: 0,
            cw20: 0,
            reward: 0,
        },
    },
};


