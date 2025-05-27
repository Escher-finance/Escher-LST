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

type SupportedNetworks = "uniontestnet" | "babylontestnet" | "babylon";


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
    liquidStakingDenom: "ebbn",
    liquidStakingDenomDisplay: "eBABY"
  },
  "babylon": {
    coinDenom: "BABY",
    coinMinimalDenom: "ubbn",
    coinDecimals: 6,
    liquidStakingDenom: "ebbn",
    liquidStakingDenomDisplay: "eBABY"
  }
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
      lst: "bbn1ug4tume0pw6d4u7r6rhae6cp3udyrv7cr0angx8qegw7ur25sdxq4krcss",
      cw20: "bbn1cnx34p82zngq0uuaendsne0x4s5gsm7gpwk2es8zk8rz8tnj938qqyq8f9",
      reward: "bbn1ug4tume0pw6d4u7r6rhae6cp3udyrv7cr0angx8qegw7ur25sdxq4krcss",
    },
    gasPrice: "0.0025ubbn",
  },
  "babylon": {
    chainId: "bbn-1",
    chainName: "babylon-mainnet",
    rest: "https://babylon-api.polkachu.com/",
    rpc: "https://babylon-rpc.polkachu.com/",
    stakeCurrency: currency["babylon"],
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
    currencies: [currency["babylon"]],
    feeCurrencies: [currency["babylon"]],
    contracts: {
      lst: "bbn1m7zr5jw4k9z22r9ajggf4ucalwy7uxvu9gkw6tnsmv42lvjpkwasagek5g",
      cw20: "bbn1s7jzz7cyuqmy5xpr07yepka5ngktexsferu2cr4xeww897ftj77sv30f5s",
      reward: "bbn1m7zr5jw4k9z22r9ajggf4ucalwy7uxvu9gkw6tnsmv42lvjpkwasagek5g",
    },
    gasPrice: "0.0025ubbn",
  },
};

export const BaseNetworks: Record<SupportedNetworks, ChainConfig> = {
  "uniontestnet": Networks["uniontestnet"],
  "babylontestnet": Networks["babylontestnet"],
  "babylon": Networks["babylon"],
};

export default Networks;

//old babylon cw20: bbn1s5qwgvzzvs5h2wurz7mjwmc4n650g3207caddlz35fay8cl5ay6ss86ejy
//old babylon lst : bbn1qmayg959zunza00s040ppqesf7qnvusys3r2m9vw35ry28x9sncq84jphy



