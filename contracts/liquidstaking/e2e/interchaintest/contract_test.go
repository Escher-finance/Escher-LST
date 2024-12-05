package main

import (
	"context"
	"fmt"
	"testing"
	"time"

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

	validator1, err := s.ChainA.Validators[0].KeyBech32(ctx, "validator", "val")
	validator2, err := s.ChainA.Validators[1].KeyBech32(ctx, "validator", "val")

	val1 := liquidstaking.Validator{
		Address: &validator1,
		Weight:  1,
	}

	val2 := liquidstaking.Validator{
		Address: &validator2,
		Weight:  1,
	}

	validators := []liquidstaking.Validator{val1, val2}

	nativeDenom := s.ChainA.Config().Denom
	lstDenom := "lmuno"

	revenueReceiver := s.UserA.FormattedAddress()

	ucs01Channel := "abc"
	ucs01RelayContract := "def"

	// Instantiate the contract with channel:
	instantiateMsg := liquidstaking.InstantiateMsg{
		UnderlyingCoinDenom: nativeDenom,
		LiquidstakingDenom:  lstDenom,
		Ucs01Channel:        ucs01Channel,
		Ucs01RelayContract:  ucs01RelayContract,
		FeeRate:             "0.1",
		RevenueReceiver:     revenueReceiver,
		UnbondingTime:       10000,
		Validators:          validators,
	}

	s.Contract, err = types.Instantiate[liquidstaking.InstantiateMsg, liquidstaking.ExecuteMsg, liquidstaking.QueryMsg](ctx, s.UserA.KeyName(), codeId, s.ChainA, instantiateMsg, "--gas", "auto")
	s.Require().NoError(err)

	minter := cw20.MinterResponse{
		Minter: s.Contract.Address,
	}

	coin := cw20.Cw20Coin{
		Amount:  "1000000000000",
		Address: s.UserA.FormattedAddress(),
	}
	initialBalances := []cw20.Cw20Coin{coin}
	cw20InstantiateMsg := cw20.InstantiateMsg{
		Name:            "lMuno",
		Symbol:          "lmuno",
		Decimals:        6,
		InitialBalances: initialBalances,
		Mint:            &minter,
	}

	s.Cw20Contract, err = types.Instantiate[cw20.InstantiateMsg, cw20.ExecuteMsg, cw20.QueryMsg](ctx, s.UserA.KeyName(), lstCodeId, s.ChainA, cw20InstantiateMsg, "--gas", "auto")
	s.Require().NoError(err)

	cw20Address := s.Cw20Contract.Address

	setParameters := liquidstaking.ExecuteMsg_SetParameters{CW20Address: &cw20Address}
	executeMsg := liquidstaking.ExecuteMsg{SetParameters: &setParameters}
	_, err = s.Contract.Execute(ctx, s.UserA.KeyName(), executeMsg, "--gas", "auto")
	s.Require().NoError(err)
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

		// var parameters liquidstaking.ParametersResponse
		// err = s.Contract.Query(ctx, liquidstaking.QueryMsg{Parameters: &liquidstaking.QueryMsg_Parameters{}}, &parameters)
		// s.Require().NoError(err)

		// s.Logger.Info(fmt.Sprintf("liquid staking params %+v\n", parameters))

		var validators liquidstaking.ValidatorsRegistry
		err = s.Contract.Query(ctx, liquidstaking.QueryMsg{Validators: &liquidstaking.QueryMsg_Validators{}}, &validators)
		s.Require().NoError(err)

		s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> VALIDATORS >>>>>>>>>>>>> %v", validators))

		BBalance, err := s.ChainA.BankQueryAllBalances(ctx, s.UserB.FormattedAddress())
		s.Require().NoError(err)
		s.Logger.Info(fmt.Sprintf("::::::::::: User B balance initially : %+v token", BBalance))

		var balance cw20.BalanceResponse
		// err = s.Cw20Contract.Query(ctx, cw20.QueryMsg{Balance: &cw20.QueryMsg_Balance{Address: s.UserA.FormattedAddress()}}, &balance)
		// s.Require().NoError(err)
		// s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> Before staking ::: User A balance %s: %s lmuno", s.UserA.FormattedAddress(), balance))

		err = s.Cw20Contract.Query(ctx, cw20.QueryMsg{Balance: &cw20.QueryMsg_Balance{Address: s.UserB.FormattedAddress()}}, &balance)
		s.Require().NoError(err)

		s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> Before staking ::: User B balance %s: %s lmuno", s.UserB.FormattedAddress(), balance))

		executeMsg := liquidstaking.ExecuteMsg{Bond: &liquidstaking.ExecuteMsg_Bond{}}
		_, err = s.Contract.Execute(ctx, s.UserB.KeyName(), executeMsg, "--amount", "800000000token", "--gas", "auto")
		s.Require().NoError(err)

		//s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> Bond Response From User B %s: %+v", s.UserB.FormattedAddress(), resp))

		err = s.Cw20Contract.Query(ctx, cw20.QueryMsg{Balance: &cw20.QueryMsg_Balance{Address: s.UserB.FormattedAddress()}}, &balance)
		s.Require().NoError(err)
		s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> After staking ::: User B balance %s: %s lmuno", s.UserB.FormattedAddress(), balance))

		BBalance, err = s.ChainA.BankQueryAllBalances(ctx, s.UserB.FormattedAddress())
		s.Require().NoError(err)
		s.Logger.Info(fmt.Sprintf("::::::::::: User B After staking : %+v token", BBalance))

		// ensure there are staking delegation appears
		delegations, err := s.ChainA.StakingQueryDelegations(ctx, s.Contract.Address)
		s.Require().NoError(err)
		s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> Delegations From User B %s: %+v", s.UserB.FormattedAddress(), delegations))

		time.Sleep(20 * time.Second)
		validator1, err := s.ChainA.Validators[0].KeyBech32(ctx, "validator", "val")
		validator2, err := s.ChainA.Validators[1].KeyBech32(ctx, "validator", "val")
		var bondAmount liquidstaking.TotalBond

		validators_addr := []string{validator1, validator2}
		err = s.Contract.Query(ctx, liquidstaking.QueryMsg{TotalBondAmount: &liquidstaking.QueryMsg_TotalBondAmount{Delegator: s.Contract.Address, Denom: s.ChainA.Config().Denom, Validators: validators_addr}}, &bondAmount)
		s.Require().NoError(err)
		s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> Contract TotalBondAmount:  %+v", bondAmount))

		// var state liquidstaking.State
		// err = s.Contract.Query(ctx, liquidstaking.QueryMsg{State: &liquidstaking.QueryMsg_State{}}, &state)
		// s.Require().NoError(err)
		// s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> Contract State:  %+v", state))

		// _, err = s.Contract.Execute(ctx, s.UserC.KeyName(), executeMsg, "--amount", "500000token", "--gas", "auto")
		// s.Require().NoError(err)

		// _, err = s.Contract.Execute(ctx, s.UserD.KeyName(), executeMsg, "--amount", "200000token", "--gas", "auto")
		// s.Require().NoError(err)

		// err = s.Contract.Query(ctx, liquidstaking.QueryMsg{State: &liquidstaking.QueryMsg_State{}}, &state)
		// s.Require().NoError(err)
		// s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> Contract State:  %+v", state))
	})

	s.Run("TestProcessRewards", func() {
		s.Logger.Info("TestProcessRewards")

		var state liquidstaking.State
		err := s.Contract.Query(ctx, liquidstaking.QueryMsg{State: &liquidstaking.QueryMsg_State{}}, &state)
		s.Require().NoError(err)
		s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> Contract State:  %+v", state))

		executeMsg := liquidstaking.ExecuteMsg{ProcessRewards: &liquidstaking.ExecuteMsg_ProcessRewards{}}
		_, err = s.Contract.Execute(ctx, s.UserA.KeyName(), executeMsg, "--gas", "auto")
		s.Require().NoError(err)

		validator1, err := s.ChainA.Validators[0].KeyBech32(ctx, "validator", "val")
		validator2, err := s.ChainA.Validators[1].KeyBech32(ctx, "validator", "val")
		validators_addr := []string{validator1, validator2}
		var bondAmount liquidstaking.TotalBond
		err = s.Contract.Query(ctx, liquidstaking.QueryMsg{TotalBondAmount: &liquidstaking.QueryMsg_TotalBondAmount{Delegator: s.Contract.Address, Denom: s.ChainA.Config().Denom, Validators: validators_addr}}, &bondAmount)
		s.Require().NoError(err)
		s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> Contract TotalBondAmount:  %+v", bondAmount))

		err = s.Contract.Query(ctx, liquidstaking.QueryMsg{State: &liquidstaking.QueryMsg_State{}}, &state)
		s.Require().NoError(err)
		s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> Contract State:  %+v", state))
	})

	// s.Run("TestUnbond", func() {
	// 	s.Logger.Info("TestUnbondBond")
	// 	var unbond_amount liquidstaking.Uint128 = "500000"

	// 	var state liquidstaking.State
	// 	err := s.Contract.Query(ctx, liquidstaking.QueryMsg{State: &liquidstaking.QueryMsg_State{}}, &state)
	// 	s.Require().NoError(err)
	// 	s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> Contract State:  %+v", state))

	// 	cw20ExecuteMsg := cw20.ExecuteMsg{Transfer: &cw20.ExecuteMsg_Transfer{Amount: unbond_amount, Recipient: s.Contract.Address}}
	// 	_, err = s.Cw20Contract.Execute(ctx, s.UserB.KeyName(), cw20ExecuteMsg, "--gas", "auto")
	// 	s.Require().NoError(err)

	// 	var balance cw20.BalanceResponse
	// 	err = s.Cw20Contract.Query(ctx, cw20.QueryMsg{Balance: &cw20.QueryMsg_Balance{Address: s.Contract.Address}}, &balance)
	// 	s.Require().NoError(err)
	// 	s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> Before unbond ::: Contract %s: %s lmuno", s.Contract.Address, balance))

	// 	executeMsg := liquidstaking.ExecuteMsg{Unbond: &liquidstaking.ExecuteMsg_Unbond{Amount: &unbond_amount}}
	// 	_, err = s.Contract.Execute(ctx, s.UserB.KeyName(), executeMsg, "--gas", "auto")
	// 	s.Require().NoError(err)

	// 	// ensure unbonding delegation appears
	// 	unbondings, err := s.ChainA.StakingQueryUnbondingDelegations(ctx, s.Contract.Address)
	// 	s.Require().NoError(err)

	// 	s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> UnbondingDelegations:  %+v", unbondings))

	// 	err = s.Cw20Contract.Query(ctx, cw20.QueryMsg{Balance: &cw20.QueryMsg_Balance{Address: s.Contract.Address}}, &balance)
	// 	s.Require().NoError(err)
	// 	s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> After unbond ::: Contract %s: %s lmuno", s.Contract.Address, balance))

	// 	err = s.Contract.Query(ctx, liquidstaking.QueryMsg{State: &liquidstaking.QueryMsg_State{}}, &state)
	// 	s.Require().NoError(err)
	// 	s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> Contract State:  %+v", state))

	// 	userB := s.UserB.FormattedAddress()

	// 	var unbondRecords []liquidstaking.UnbondRecord
	// 	False := false
	// 	err = s.Contract.Query(ctx, liquidstaking.QueryMsg{UnbondRecord: &liquidstaking.QueryMsg_UnbondRecord{Staker: &userB, Released: &False}}, &unbondRecords)
	// 	s.Require().NoError(err)
	// 	s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> UnbondRecords:  %+v", unbondRecords))

	// 	// ensure unbonding delegation appears
	// 	unbondings, err = s.ChainA.StakingQueryUnbondingDelegations(ctx, s.Contract.Address)
	// 	s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> UnbondingDelegations before sleep:  %+v", unbondings))

	// 	s.Logger.Info("Sleep 10 secs")

	// 	time.Sleep(10 * time.Second)

	// 	s.Logger.Info("Check unbonding delegations again")

	// 	// ensure unbonding delegation appears
	// 	unbondings, err = s.ChainA.StakingQueryUnbondingDelegations(ctx, s.Contract.Address)
	// 	s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> UnbondingDelegations:  %+v", unbondings))

	// 	BBalances, err := s.ChainA.BankQueryAllBalances(ctx, s.UserB.FormattedAddress())
	// 	s.Require().NoError(err)
	// 	s.Logger.Info(fmt.Sprintf(":::::::::: User B balance before process unbonding : %+v", BBalances))

	// 	executeMsg = liquidstaking.ExecuteMsg{ProcessUnbonding: &liquidstaking.ExecuteMsg_ProcessUnbonding{Id: 1}}
	// 	_, err = s.Contract.Execute(ctx, s.UserA.KeyName(), executeMsg, "--gas", "auto")
	// 	s.Require().NoError(err)
	// 	//s.Logger.Info(fmt.Sprintf("::::::::::: Process Unbonding Response : %+v", res))

	// 	err = testutil.WaitForBlocks(ctx, 3, s.ChainA)
	// 	s.Require().NoError(err)

	// 	BBalances, err = s.ChainA.BankQueryAllBalances(ctx, s.UserB.FormattedAddress())
	// 	s.Require().NoError(err)
	// 	s.Logger.Info(fmt.Sprintf("::::::::::: User B balance after process unbonding : %+v", BBalances))

	// 	err = s.Contract.Query(ctx, liquidstaking.QueryMsg{UnbondRecord: &liquidstaking.QueryMsg_UnbondRecord{Staker: &userB}}, &unbondRecords)
	// 	s.Require().NoError(err)
	// 	s.Logger.Info(fmt.Sprintf(">>>>>>>>>>>> UnbondRecords:  %+v", unbondRecords))
	// })

}
