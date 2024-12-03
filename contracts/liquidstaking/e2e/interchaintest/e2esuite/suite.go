package e2esuite

import (
	"context"

	"cosmossdk.io/math"
	dockerclient "github.com/docker/docker/client"
	"github.com/stretchr/testify/suite"
	"go.uber.org/zap"
	"go.uber.org/zap/zaptest"

	interchaintest "github.com/strangelove-ventures/interchaintest/v8"
	"github.com/strangelove-ventures/interchaintest/v8/chain/cosmos"
	"github.com/strangelove-ventures/interchaintest/v8/ibc"
	"github.com/strangelove-ventures/interchaintest/v8/testreporter"
	"github.com/strangelove-ventures/interchaintest/v8/testutil"
)

// TestSuite is a suite of tests that require two chains and a relayer
type TestSuite struct {
	suite.Suite

	ChainA *cosmos.CosmosChain
	UserA  ibc.Wallet
	UserB  ibc.Wallet
	UserC  ibc.Wallet
	UserD  ibc.Wallet

	ChainAConnID    string
	ChainAChannelID string
	dockerClient    *dockerclient.Client
	network         string
	Logger          *zap.Logger
	ExecRep         *testreporter.RelayerExecReporter
}

// SetupSuite sets up the chains, relayer, user accounts, clients, and connections
func (s *TestSuite) SetupSuite(ctx context.Context, chainSpecs []*interchaintest.ChainSpec) {
	if len(chainSpecs) < 1 {
		panic("ContractTestSuite requires exactly min 1 chain specs")
	}

	t := s.T()

	s.Logger = zaptest.NewLogger(t)
	s.dockerClient, s.network = interchaintest.DockerSetup(t)

	cf := interchaintest.NewBuiltinChainFactory(s.Logger, chainSpecs)

	chains, err := cf.Chains(t.Name())
	s.Require().NoError(err)
	s.ChainA = chains[0].(*cosmos.CosmosChain)

	ic := interchaintest.NewInterchain().AddChain(s.ChainA)

	s.Require().NoError(ic.Build(ctx, s.ExecRep, interchaintest.InterchainBuildOptions{
		TestName:         t.Name(),
		Client:           s.dockerClient,
		NetworkID:        s.network,
		SkipPathCreation: true,
	}))

	// Fund a user account on ChainA
	userFunds := math.NewInt(100_000_000)
	users := interchaintest.GetAndFundTestUsers(t, ctx, t.Name(), userFunds, s.ChainA)
	s.UserA = users[0]

	mnemonicB := "manual champion squirrel price purchase space evidence media absurd portion sick float"
	s.UserB, err = interchaintest.GetAndFundTestUserWithMnemonic(ctx, t.Name(), mnemonicB, userFunds, s.ChainA)
	s.Require().NoError(err)

	userCFunds := math.NewInt(1_000_000)
	mnemonicC := "between swallow already bean morning peasant half damage display win brick grunt immense lizard poem visa move fence other drip power siege diary ahead"
	s.UserC, err = interchaintest.GetAndFundTestUserWithMnemonic(ctx, t.Name(), mnemonicC, userCFunds, s.ChainA)
	s.Require().NoError(err)

	userDFunds := math.NewInt(100_000_000)
	mnemonicD := "always join mean salon bachelor circle truck silk base deal inquiry solar"
	s.UserD, err = interchaintest.GetAndFundTestUserWithMnemonic(ctx, t.Name(), mnemonicD, userDFunds, s.ChainA)
	s.Require().NoError(err)

	err = testutil.WaitForBlocks(ctx, 2, s.ChainA)
	s.Require().NoError(err)

	t.Cleanup(
		func() {
			// Collect diagnostics
			chains := []string{chainSpecs[0].ChainConfig.Name}
			collect(t, s.dockerClient, true, chains...)
		},
	)
}
