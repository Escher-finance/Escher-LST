package chainconfig

import (
	"github.com/Escher-finance/evm-union-liquid-staking/interchaintest/e2esuite"
	interchaintest "github.com/strangelove-ventures/interchaintest/v8"
	"github.com/strangelove-ventures/interchaintest/v8/chain/cosmos"
	"github.com/strangelove-ventures/interchaintest/v8/ibc"
)

func create(x int64) *int64 {
	return &x
}

func createInt(x int) *int {
	return &x
}

var DefaultChainSpecs = []*interchaintest.ChainSpec{
	// -- WASMD --
	{
		NumValidators: createInt(3),
		ChainConfig: ibc.ChainConfig{
			Type:    "cosmos",
			Name:    "wasmd",
			ChainID: "wasmd-1",
			Images: []ibc.DockerImage{
				{
					Repository: "cosmwasm/wasmd", // FOR LOCAL IMAGE USE: Docker Image Name
					Version:    "v0.52.0",        // FOR LOCAL IMAGE USE: Docker Image Tag
					UidGid:     "1025:1025",
				},
			},
			Bin:            "wasmd",
			Bech32Prefix:   "wasm",
			Denom:          "token",
			CoinDecimals:   create(6),
			GasPrices:      "0.0token",
			GasAdjustment:  1.3,
			EncodingConfig: e2esuite.EncodingConfig(),
			TrustingPeriod: "508h",
			NoHostMount:    false,
			ModifyGenesis:  cosmos.ModifyGenesis([]cosmos.GenesisKV{cosmos.NewGenesisKV("app_state.staking.params.unbonding_time", "4s")}),
		},
	},
}
