export type Currency = {
  coinDenom: string;
  coinMinimalDenom: string;
  coinDecimals: number;
};

export type CoinType = {
  coinType: number;
};

export type ChainConfig = {
  chainId: string;
  chainName: string;
  rpc: string;
  rest: string;
  stakeCurrency: Currency;
  bip44: CoinType;
  bech32Config: {
    bech32PrefixAccAddr: string;
    bech32PrefixAccPub: string;
    bech32PrefixValAddr: string;
    bech32PrefixValPub: string;
    bech32PrefixConsAddr: string;
    bech32PrefixConsPub: string;
  };
  currencies: Currency[];
  feeCurrencies: Currency[];
  contracts: {
    lst: string;
    cw20: string;
    reward: string;
  };
  gasPrice: string;
  connectionID: string;
  counterPartyConnectionID: string;
  counterpartyPortID: string;
};

type SupportedNetworks = "lst-network";


const currency: Record<SupportedNetworks, Currency> = {

  "lst-network": {
    coinDenom: "STK",
    coinMinimalDenom: "stake",
    coinDecimals: 6,
  },
};

const Networks: Record<SupportedNetworks, ChainConfig> = {

  "lst-network": {
    chainId: "lst",
    chainName: "LST Network",
    rpc: "https://lst.rickyanto.com",
    rest: "https://lst.rickyanto.com",
    stakeCurrency: currency["lst-network"],
    bip44: {
      coinType: 118
    },
    bech32Config: {
      bech32PrefixAccAddr: "cosmos",
      bech32PrefixAccPub: "cosmospub",
      bech32PrefixValAddr: "cosmosvaloper",
      bech32PrefixValPub: "cosmosvaloperpub",
      bech32PrefixConsAddr: "cosmosvalcons",
      bech32PrefixConsPub: "cosmosvalconspub",
    },
    currencies: [currency["lst-network"]],
    feeCurrencies: [currency["lst-network"]],
    contracts: {
      lst: "cosmos1c2f79k5kykr5s4zhknn5w56hs5c9a8zxh4w03x07dnzwyrcj4pfspyf0pr",
      cw20: "cosmos1t3f4zxve6725sf4glrnlar8uku78j0nyfl0ppzgfju9ft9phvqwqren6rp",
      reward: "cosmos1pd7kfgvr5tpcv0xnlv46c4jsq9jg2r799xxrcwqdm4l2jhq2pjwqyg4puh",
    },
    gasPrice: "0.01stake",
    connectionID: "connection-26",
    counterPartyConnectionID: "connection-5",
    counterpartyPortID: "0x9f48D6e0Ab40dF6FB0bE0e96e80971441CEf3787"
  },
};

export const BaseNetworks: Record<SupportedNetworks, ChainConfig> = {
  "lst-network": Networks["lst-network"],
};

export default Networks;