package main

import (
	"context"
	"fmt"
	"testing"

	"github.com/stretchr/testify/suite"

	"github.com/strangelove-ventures/interchaintest/v8"
	"github.com/strangelove-ventures/interchaintest/v8/testutil"

	"github.com/Escher-finance/evm-union-liquid-staking/interchaintest/chainconfig"
	"github.com/Escher-finance/evm-union-liquid-staking/interchaintest/e2esuite"
	"github.com/Escher-finance/evm-union-liquid-staking/interchaintest/types"
	"github.com/Escher-finance/evm-union-liquid-staking/interchaintest/types/cw20"
	"github.com/Escher-finance/evm-union-liquid-staking/interchaintest/types/liquidstaking"
)

type ContractTestSuite struct {
	e2esuite.TestSuite

	// Contract is the representation of the ICQ controller contract
	Contract *types.Contract[
		liquidstaking.InstantiateMsg, liquidstaking.ExecuteMsg, liquidstaking.QueryMsg,
	]

	Cw20Contract *types.Contract[
		cw20.InstantiateMsg, cw20.ExecuteMsg, cw20.QueryMsg,
	]
}

// SetupSuite calls the underlying TestSuite's SetupSuite method
func (s *ContractTestSuite) SetupSuite(ctx context.Context, chainSpecs []*interchaintest.ChainSpec) {
	s.TestSuite.SetupSuite(ctx, chainSpecs)
}

// SetupContractTestSuite starts the chains, relayer, creates the user accounts, creates the ibc clients and connections,
// sets up the contract and does the channel handshake for the contract test suite.
func (s *ContractTestSuite) SetupContractTestSuite(ctx context.Context) {
	fmt.Println("SetupContractTestSuite")
	s.SetupSuite(ctx, chainconfig.DefaultChainSpecs)
	s.Logger.Info("After SetupContractTestSuite")

	// // Wait for the block to be ready
	err := testutil.WaitForBlocks(ctx, 5, s.ChainA)
	s.Require().NoError(err)

	lstCodeId, err := s.ChainA.StoreContract(ctx, s.UserA.KeyName(), "../../artifacts/cw20_base.wasm")
	s.Require().NoError(err)

	codeId, err := s.ChainA.StoreContract(ctx, s.UserA.KeyName(), "../../artifacts/evm_union_liquid_staking.wasm")
	s.Require().NoError(err)

	validator, err := s.ChainA.Validators[0].KeyBech32(ctx, "validator", "val")

	val1 := liquidstaking.Validator{
		Address: &validator,
		Weight:  1,
	}

	validators := []liquidstaking.Validator{val1}

	nativeDenom := s.ChainA.Config().Denom
	lstDenom := "sttoken"

	revenueReceiver := s.UserB.FormattedAddress()

	ucs01Channel := "abc"
	ucs01RelayContract := "def"
	//cw20address := "123"

	// Instantiate the contract with channel:
	instantiateMsg := liquidstaking.InstantiateMsg{
		UnderlyingCoinDenom: nativeDenom,
		LiquidstakingDenom:  lstDenom,
		Ucs01Channel:        ucs01Channel,
		Ucs01RelayContract:  ucs01RelayContract,
		FeeRate:             "100000000000000000",
		RevenueReceiver:     revenueReceiver,
		UnbondingTime:       4000,
		Validators:          validators,
	}

	s.Contract, err = types.Instantiate[liquidstaking.InstantiateMsg, liquidstaking.ExecuteMsg, liquidstaking.QueryMsg](ctx, s.UserA.KeyName(), codeId, s.ChainA, instantiateMsg, "--gas", "500000")
	s.Require().NoError(err)

	minter := cw20.MinterResponse{
		Minter: s.Contract.Address,
	}

	coin := cw20.Cw20Coin{
		Amount:  "100000000000000000000000",
		Address: s.UserA.FormattedAddress(),
	}
	initialBalances := []cw20.Cw20Coin{coin}
	cw20InstantiateMsg := cw20.InstantiateMsg{
		Name:            "lMuno",
		Symbol:          "lmuno",
		Decimals:        18,
		InitialBalances: initialBalances,
		Mint:            &minter,
	}

	s.Cw20Contract, err = types.Instantiate[cw20.InstantiateMsg, cw20.ExecuteMsg, cw20.QueryMsg](ctx, s.UserA.KeyName(), lstCodeId, s.ChainA, cw20InstantiateMsg, "--gas", "500000")
	s.Require().NoError(err)

	cw20Address := s.Cw20Contract.Address

	setParameters := liquidstaking.ExecuteMsg_SetParameters{CW20Address: &cw20Address}
	executeMsg := liquidstaking.ExecuteMsg{SetParameters: &setParameters}
	resp, err := s.Contract.Execute(ctx, s.UserA.KeyName(), executeMsg, "--gas", "auto")
	s.Require().NoError(err)

	s.Logger.Info(fmt.Sprintf("resp %+v\n", resp))
}

func TestWithContractTestSuite(t *testing.T) {
	suite.Run(t, new(ContractTestSuite))
}

func (s *ContractTestSuite) TestContractBond() {
	s.ContractBondTest()
}

func (s *ContractTestSuite) ContractBondTest() {
	ctx := context.Background()

	// This starts the chains, relayer, creates the user accounts, creates the ibc clients and connections,
	// sets up the contract and does the channel handshake for the contract test suite.
	s.SetupContractTestSuite(ctx)

	s.Run("TestBond", func() {
		s.Logger.Info("TestBond")

		cmd := []string{"wasmd", "query", "staking", "params",
			"--node", s.ChainA.GetRPCAddress(),
			"--output", "json",
		}
		stdout, _, err := s.ChainA.Exec(ctx, cmd, nil)
		s.Require().NoError(err)
		str := string(stdout)
		s.Logger.Info(fmt.Sprintf("staking params %s\n", str))

		var parameters liquidstaking.ParametersResponse
		err = s.Contract.Query(ctx, liquidstaking.QueryMsg{Parameters: &liquidstaking.QueryMsg_Parameters{}}, &parameters)
		s.Require().NoError(err)

		s.Logger.Info(fmt.Sprintf("liquid staking params %+v\n", parameters))

		executeMsg := liquidstaking.ExecuteMsg{Bond: &liquidstaking.ExecuteMsg_Bond{}}
		resp, err := s.Contract.Execute(ctx, s.UserA.KeyName(), executeMsg, "--amount", "1000000000token", "--gas", "auto")
		s.Require().NoError(err)
		s.Logger.Info(fmt.Sprintf("resp %+v\n", resp))

	})

}
