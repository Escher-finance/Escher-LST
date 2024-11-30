package main

import (
	"context"
	"fmt"
	"testing"

	"github.com/stretchr/testify/suite"

	"github.com/strangelove-ventures/interchaintest/v8"

	"github.com/Escher-finance/evm-union-liquid-staking/interchaintest/chainconfig"
	"github.com/Escher-finance/evm-union-liquid-staking/interchaintest/e2esuite"
	"github.com/Escher-finance/evm-union-liquid-staking/interchaintest/types"
	"github.com/Escher-finance/evm-union-liquid-staking/interchaintest/types/liquidstaking"
)

type ContractTestSuite struct {
	e2esuite.TestSuite

	// Contract is the representation of the ICQ controller contract
	Contract *types.Contract[
		liquidstaking.InstantiateMsg, liquidstaking.ExecuteMsg, liquidstaking.QueryMsg,
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
	})
}
