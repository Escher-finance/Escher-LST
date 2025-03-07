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
};

type SupportedNetworks = "uniontestnet" | "babylontestnet";


const currency: Record<SupportedNetworks, Currency> = {
  "uniontestnet": {
    coinDenom: "UNO",
    coinMinimalDenom: "muno",
    coinDecimals: 6,
    liquidStakingDenom: "emuno",
    liquidStakingDenomDisplay: "eMUNO"
  },
  "babylontestnet": {
    coinDenom: "BABY",
    coinMinimalDenom: "ubbn",
    coinDecimals: 6,
    liquidStakingDenom: "ubbn",
    liquidStakingDenomDisplay: "eBABY"
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
      lst: "union1mdsv9vd9f0gjte83vauwjqsahxg4gte2mdkcxxex68p97h8a4txqq0k5ct",
      cw20: "union1uf2jmjgaxwdxl5ttnwcef829lm2hgcxcxczyn93leuuhs4jrtm8sgse85m",
      reward: "union14nt98pl3edsgd4lu56m3yndervtp9z3qvyp0wmqkx6tmmse5ufnsrct8pc",
    },
    gasPrice: "0.0025muno"
  },
  "babylontestnet": {
    chainId: "bbn-test-5",
    chainName: "babylontestnet",
    rest: "https://babylon-testnet-api.nodes.guru",
    rpc: "https://babylon-testnet-rpc.nodes.guru",
    stakeCurrency: currency["babylontestnet"],
    bip44: {
      coinType: 118
    },
    bech32Config: {
      bech32PrefixAccAddr: "bbn",
      bech32PrefixAccPub: "bbnpub",
      bech32PrefixValAddr: "bbnvaloper",
      bech32PrefixValPub: "bbnvaloperpub",
      bech32PrefixConsAddr: "bbnvalcons",
      bech32PrefixConsPub: "bbnvalconspub",
    },
    currencies: [currency["babylontestnet"]],
    feeCurrencies: [currency["babylontestnet"]],
    contracts: {
      lst: "bbn18ed0qdj7nvytfvc6rftgryk4krxxw4k2ql7e7f7z3t747828czrs52vxaa",
      cw20: "bbn144hnwjtykzje3r4eccszq33fegycymh680huylagm3tqrwxhjrjqvkul3y",
      reward: "bbn18ed0qdj7nvytfvc6rftgryk4krxxw4k2ql7e7f7z3t747828czrs52vxaa",
    },
    gasPrice: "0.0025ubbn",
  },
};

export const BaseNetworks: Record<SupportedNetworks, ChainConfig> = {
  "uniontestnet": Networks["uniontestnet"],
  "babylontestnet": Networks["babylontestnet"],
};

export default Networks;

