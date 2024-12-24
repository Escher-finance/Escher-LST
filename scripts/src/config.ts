export type ChainConfig = {
    chainId: string;
    chainName: string;
    coinDenom: string,
    lstCoinDenom: string,
    lstCoinSymbol: string,
    rpc: string;
    rest: string;
    ucs01Channel: string;
    ucs01RelayContract: string;
    feeRate: string;
    revenueReceiver: string;
    validator: string;
    codeId: {
        lst: number | undefined;
        cw20: number | undefined;
        reward: number | undefined;
    };
};

export type SupportedNetworks = "lst-network" | "union-testnet-9";


export const Networks: Record<SupportedNetworks, ChainConfig> = {
    "lst-network": {
        chainId: "lst",
        chainName: "LST Network",
        rpc: "https://lst.rickyanto.com",
        rest: "https://lst.rickyanto.com",
        coinDenom: "stake",
        lstCoinDenom: "lqstake",
        lstCoinSymbol: "LQSTAKE",
        ucs01Channel: "channel-86",
        ucs01RelayContract: "union1m87a5scxnnk83wfwapxlufzm58qe2v65985exff70z95a2yr86yq7hl08h",
        feeRate: "0.1",
        revenueReceiver: "cosmos1pss37x3hwq5ytk7uhf9fjcpcd7k20pekq6xtlz",
        validator: "cosmosvaloper1h492ust5a9qzhh4zhhhlyva9v8ftn5sz99k4yp",
        codeId: {
            lst: 21,
            cw20: 20,
            reward: 22,
        },
    },
    "union-testnet-9": {
        chainId: "lst",
        chainName: "LST Network",
        rpc: "https://rpc.testnet-9.union.build:443",
        rest: "https://api.testnet-9.union.build:443",
        coinDenom: "stake",
        lstCoinDenom: "lqstake",
        lstCoinSymbol: "LQSTAKE",
        ucs01Channel: "",
        ucs01RelayContract: "",
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


