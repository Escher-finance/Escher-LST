export type ChainConfig = {
    chainId: string;
    chainName: string;
    coinDenom: string,
    lstCoinDenom: string,
    lstCoinSymbol: string,
    rpc: string;
    rest: string;
    ucs03Channel: number;
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
        ucs03Channel: 2,
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
        chainId: "union-testnet-10",
        chainName: "uniontestnet",
        rpc: "https://rpc.rpc-node.union-testnet-10.union.build",
        rest: "https://api.rpc-node.union-testnet-10.union.build",
        coinDenom: "au",
        lstCoinDenom: "eau",
        lstCoinSymbol: "eAU",
        ucs03Channel: 1,
        ucs03RelayContract: "union1336jj8ertl8h7rdvnz4dh5rqahd09cy0x43guhsxx6xyrztx292qpe64fh",
        feeRate: "0.1",
        revenueReceiver: "union1vnglhewf3w66cquy6hr7urjv3589srheampz42",
        validator: "unionvaloper14qekdkj2nmmwea4ufg9n002a3pud23y87mnkjg",
        codeId: {
            lst: 156,
            cw20: 155,
            reward: 157,
        },
    },
};


