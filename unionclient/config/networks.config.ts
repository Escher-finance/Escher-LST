export type Currency = {
  coinDenom: string;
  coinMinimalDenom: string;
  coinDecimals: number;
  liquidStakingDenom: string;
  liquidStakingDenomDisplay: string;
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

type SupportedNetworks = "uniontestnet";


const currency: Record<SupportedNetworks, Currency> = {
  "uniontestnet": {
    coinDenom: "UNO",
    coinMinimalDenom: "muno",
    coinDecimals: 6,
    liquidStakingDenom: "factory/union1vnglhewf3w66cquy6hr7urjv3589srheampz42/limuno",
    liquidStakingDenomDisplay: "limuno"
  },
};

const Networks: Record<SupportedNetworks, ChainConfig> = {

  "uniontestnet": {
    chainId: "union-testnet-9",
    chainName: "uniontestnet",
    rest: "https://rest.testnet-9.union.build",
    rpc: "https://rpc.testnet-9.union.build",
    stakeCurrency: currency["uniontestnet"],
    bip44: {
      coinType: 118
    },
    bech32Config: {
      bech32PrefixAccAddr: "union",
      bech32PrefixAccPub: "unionpub",
      bech32PrefixValAddr: "unionvaloper",
      bech32PrefixValPub: "unionvaloperpub",
      bech32PrefixConsAddr: "unionvalcons",
      bech32PrefixConsPub: "unionvalconspub",
    },
    currencies: [currency["uniontestnet"]],
    feeCurrencies: [currency["uniontestnet"]],
    contracts: {
      lst: "union133syr5c6czxmfg4qvd4je5rgs5cvparuems6hknw3unk0934tgdq4529n8",
      cw20: "",
      reward: "union1uey97a7mhdxf75cxxp7tfale38sdrq8vjf03qg5al80tfqwm6e0qfdfqyf",
    },
    gasPrice: "0.0025muno",
    connectionID: "connection-26",
    counterPartyConnectionID: "connection-5",
    counterpartyPortID: "0x9f48D6e0Ab40dF6FB0bE0e96e80971441CEf3787"
  },
};

export const BaseNetworks: Record<SupportedNetworks, ChainConfig> = {
  "uniontestnet": Networks["uniontestnet"],
};

export default Networks;

//factory/union1vnglhewf3w66cquy6hr7urjv3589srheampz42/limuno
// 108