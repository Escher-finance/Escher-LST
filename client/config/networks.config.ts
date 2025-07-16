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

export type Channel = {
  sourceChannelId: number;
  destinationChannelId: number;
  sourceIbcChannelId: string;
  destinationIbcChannelId: string;
}

export type EscherConfig = {
  lst: string;
  ucs03: string;
  tokenMinter: string;
  babyDenom: string;
  ebabyDenom: string;
  channel: Record<string, Channel>;
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
  escher: EscherConfig;
};

type SupportedNetworks = "babylon-testnet" | "babylon-mainnet" | "osmosis-testnet" | "osmosis-mainnet";



const currency: Record<SupportedNetworks, Currency> = {
  "babylon-testnet": {
    coinDenom: "BABY",
    coinMinimalDenom: "ubbn",
    coinDecimals: 6,
    liquidStakingDenom: "ebbn",
    liquidStakingDenomDisplay: "eBABY"
  },
  "babylon-mainnet": {
    coinDenom: "BABY",
    coinMinimalDenom: "ubbn",
    coinDecimals: 6,
    liquidStakingDenom: "ebbn",
    liquidStakingDenomDisplay: "eBABY"
  },
  "osmosis-testnet": {
    coinDenom: "OSMO",
    coinMinimalDenom: "uosmo",
    coinDecimals: 6,
    liquidStakingDenom: "ebbn",
    liquidStakingDenomDisplay: "eBABY",
  },
  "osmosis-mainnet": {
    coinDenom: "OSMO",
    coinMinimalDenom: "uosmo",
    coinDecimals: 6,
    liquidStakingDenom: "ebbn",
    liquidStakingDenomDisplay: "eBABY",
  }
};


const Networks: Record<SupportedNetworks, ChainConfig> = {

  "babylon-testnet": {
    chainId: "bbn-test-5",
    chainName: "babylon-testnet",
    rest: "https://babylon-testnet-api.nodes.guru",
    rpc: "https://babylon-testnet-rpc.nodes.guru",
    stakeCurrency: currency["babylon-testnet"],
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
    currencies: [currency["babylon-testnet"]],
    feeCurrencies: [currency["babylon-testnet"]],
    contracts: {
      lst: "bbn1ug4tume0pw6d4u7r6rhae6cp3udyrv7cr0angx8qegw7ur25sdxq4krcss",
      cw20: "bbn1cnx34p82zngq0uuaendsne0x4s5gsm7gpwk2es8zk8rz8tnj938qqyq8f9",
      reward: "bbn1ug4tume0pw6d4u7r6rhae6cp3udyrv7cr0angx8qegw7ur25sdxq4krcss",
    },
    gasPrice: "0.0025ubbn",
    escher: {
      lst: "bbn1ug4tume0pw6d4u7r6rhae6cp3udyrv7cr0angx8qegw7ur25sdxq4krcss",
      ucs03: "osmo1336jj8ertl8h7rdvnz4dh5rqahd09cy0x43guhsxx6xyrztx292qs2uecc",
      babyDenom: "bbn1cnx34p82zngq0uuaendsne0x4s5gsm7gpwk2es8zk8rz8tnj938qqyq8f9",
      ebabyDenom: "factory/osmo13ulc6pqhm60qnx58ss7s3cft8cqfycexq3uy3dd2v0l8qsnkvk4sj22sn6/5dDrk51st6AKJwxbyFwe8wydD417XHRDAAx9JSJN7c9a",
      tokenMinter: "bbn1sakazthycqgzer50nqgr5ta4vy3gwz8wxla3s8rd8pql4ctmz5qssg39sf",
      channel: {
        "osmosis": {
          sourceChannelId: 5,
          destinationChannelId: 3,
          sourceIbcChannelId: "channel-10366",
          destinationIbcChannelId: "channel-1"
        }
      }
    }
  },
  "babylon-mainnet": {
    chainId: "bbn-1",
    chainName: "babylon-mainnet",
    rest: "https://babylon-api.polkachu.com/",
    rpc: "https://babylon-rpc.polkachu.com/",
    stakeCurrency: currency["babylon-mainnet"],
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
    currencies: [currency["babylon-mainnet"]],
    feeCurrencies: [currency["babylon-mainnet"]],
    contracts: {
      lst: "bbn1m7zr5jw4k9z22r9ajggf4ucalwy7uxvu9gkw6tnsmv42lvjpkwasagek5g",
      cw20: "bbn1s7jzz7cyuqmy5xpr07yepka5ngktexsferu2cr4xeww897ftj77sv30f5s",
      reward: "bbn1m7zr5jw4k9z22r9ajggf4ucalwy7uxvu9gkw6tnsmv42lvjpkwasagek5g",
    },
    gasPrice: "0.0025ubbn",
    escher: {
      lst: "bbn1m7zr5jw4k9z22r9ajggf4ucalwy7uxvu9gkw6tnsmv42lvjpkwasagek5g",
      ucs03: "bbn1336jj8ertl8h7rdvnz4dh5rqahd09cy0x43guhsxx6xyrztx292q77945h",
      babyDenom: "ubbn",
      ebabyDenom: "bbn1s7jzz7cyuqmy5xpr07yepka5ngktexsferu2cr4xeww897ftj77sv30f5s",
      tokenMinter: "bbn1c723xf74f0r9g4uyn0cv2t7pkgcq7x0gaw5h773j78rk35w0j0usslxen6",
      channel: {
        "osmosis": {
          sourceChannelId: 4,
          destinationChannelId: 1,
          sourceIbcChannelId: "channel-3",
          destinationIbcChannelId: "channel-1"
        }
      }
    }
  },
  "osmosis-testnet": {
    chainId: "osmo-test-5",
    chainName: "Osmosis (Testnet)",
    rpc: "https://rpc.testnet.osmosis.zone",
    rest: "https://lcd.osmotest5.osmosis.zone",
    stakeCurrency: currency["osmosis-testnet"],
    bech32Config: {
      bech32PrefixAccAddr: "osmo",
      bech32PrefixAccPub: "osmopub",
      bech32PrefixValAddr: "osmovaloper",
      bech32PrefixValPub: "osmovaloperpub",
      bech32PrefixConsAddr: "osmovalcons",
      bech32PrefixConsPub: "osmovalconspub",
    },
    currencies: [currency["osmosis-testnet"]],
    feeCurrencies: [currency["osmosis-testnet"]],
    "bip44": {
      "coinType": 118
    },
    contracts: {
      lst: "bbn1ug4tume0pw6d4u7r6rhae6cp3udyrv7cr0angx8qegw7ur25sdxq4krcss",
      cw20: "bbn1s7jzz7cyuqmy5xpr07yepka5ngktexsferu2cr4xeww897ftj77sv30f5s",
      reward: "bbn1m7zr5jw4k9z22r9ajggf4ucalwy7uxvu9gkw6tnsmv42lvjpkwasagek5g",
    },
    gasPrice: "0.025uosmo",
    escher: {
      lst: "bbn1ug4tume0pw6d4u7r6rhae6cp3udyrv7cr0angx8qegw7ur25sdxq4krcss",
      ucs03: "osmo1336jj8ertl8h7rdvnz4dh5rqahd09cy0x43guhsxx6xyrztx292qs2uecc",
      babyDenom: "factory/osmo13ulc6pqhm60qnx58ss7s3cft8cqfycexq3uy3dd2v0l8qsnkvk4sj22sn6/F7BfSnXtmfRa3CGUAG8APpUWkByDvhdEpnFHtiKY9EB",
      ebabyDenom: "factory/osmo13ulc6pqhm60qnx58ss7s3cft8cqfycexq3uy3dd2v0l8qsnkvk4sj22sn6/5dDrk51st6AKJwxbyFwe8wydD417XHRDAAx9JSJN7c9a",
      tokenMinter: "osmo13ulc6pqhm60qnx58ss7s3cft8cqfycexq3uy3dd2v0l8qsnkvk4sj22sn6",
      channel: {
        "babylon": {
          sourceIbcChannelId: "channel-10366",
          sourceChannelId: 3,
          destinationChannelId: 5,
          destinationIbcChannelId: "channel-101635"
        }
      }
    }
  },
  "osmosis-mainnet": {
    chainId: "osmosis-1",
    chainName: "Osmosis (Mainnet)",
    rpc: "https://rpc.osmosis.zone",
    rest: "https://lcd.osmosis.zone",
    stakeCurrency: currency["osmosis-mainnet"],
    bech32Config: {
      bech32PrefixAccAddr: "osmo",
      bech32PrefixAccPub: "osmopub",
      bech32PrefixValAddr: "osmovaloper",
      bech32PrefixValPub: "osmovaloperpub",
      bech32PrefixConsAddr: "osmovalcons",
      bech32PrefixConsPub: "osmovalconspub",
    },
    currencies: [currency["osmosis-mainnet"]],
    feeCurrencies: [currency["osmosis-mainnet"]],
    "bip44": {
      "coinType": 118
    },
    gasPrice: "0.025uosmo",
    contracts: {
      lst: "bbn1m7zr5jw4k9z22r9ajggf4ucalwy7uxvu9gkw6tnsmv42lvjpkwasagek5g",
      cw20: "bbn1s7jzz7cyuqmy5xpr07yepka5ngktexsferu2cr4xeww897ftj77sv30f5s",
      reward: "bbn1m7zr5jw4k9z22r9ajggf4ucalwy7uxvu9gkw6tnsmv42lvjpkwasagek5g",
    },
    escher: {
      lst: "bbn1m7zr5jw4k9z22r9ajggf4ucalwy7uxvu9gkw6tnsmv42lvjpkwasagek5g",
      ucs03: "osmo1336jj8ertl8h7rdvnz4dh5rqahd09cy0x43guhsxx6xyrztx292qs2uecc",
      babyDenom: "ibc/EC3A4ACBA1CFBEE698472D3563B70985AEA5A7144C319B61B3EBDFB57B5E1535",
      ebabyDenom: "factory/osmo12r3yc76u9lxe33yemstatnw8602culdjzrtr8lmnpycmd3z7d4jsxx60kc/FwNhFaW3zLxoLUgXCdWjqBzcvGNPaB7B2XZqm2xgrB93",
      tokenMinter: "osmo12r3yc76u9lxe33yemstatnw8602culdjzrtr8lmnpycmd3z7d4jsxx60kc",
      channel: {
        "babylon": {
          sourceIbcChannelId: "channel-101635",
          sourceChannelId: 1,
          destinationChannelId: 4,
          destinationIbcChannelId: "channel-3"
        }
      }
    }
  }
};

export const BaseNetworks: Record<SupportedNetworks, ChainConfig> = {
  "babylon-testnet": Networks["babylon-testnet"],
  "babylon-mainnet": Networks["babylon-mainnet"],
  "osmosis-testnet": Networks["osmosis-testnet"],
  "osmosis-mainnet": Networks["osmosis-mainnet"],
};

export default Networks;

//old babylon cw20: bbn1s5qwgvzzvs5h2wurz7mjwmc4n650g3207caddlz35fay8cl5ay6ss86ejy
//old babylon lst : bbn1qmayg959zunza00s040ppqesf7qnvusys3r2m9vw35ry28x9sncq84jphy


// "uniontestnet": {
//   chainId: "union-testnet-9",
//   chainName: "uniontestnet",
//   rest: "https://rest.testnet-9.union.build",
//   rpc: "https://rpc.testnet-9.union.build",
//   stakeCurrency: currency["uniontestnet"],
//   bip44: {
//     coinType: 118
//   },
//   bech32Config: {
//     bech32PrefixAccAddr: "union",
//     bech32PrefixAccPub: "unionpub",
//     bech32PrefixValAddr: "unionvaloper",
//     bech32PrefixValPub: "unionvaloperpub",
//     bech32PrefixConsAddr: "unionvalcons",
//     bech32PrefixConsPub: "unionvalconspub",
//   },
//   currencies: [currency["uniontestnet"]],
//   feeCurrencies: [currency["uniontestnet"]],
//   contracts: {
//     lst: "union1mdsv9vd9f0gjte83vauwjqsahxg4gte2mdkcxxex68p97h8a4txqq0k5ct",
//     cw20: "union1uf2jmjgaxwdxl5ttnwcef829lm2hgcxcxczyn93leuuhs4jrtm8sgse85m",
//     reward: "union14nt98pl3edsgd4lu56m3yndervtp9z3qvyp0wmqkx6tmmse5ufnsrct8pc",
//   },
//   gasPrice: "0.0025muno"
// },