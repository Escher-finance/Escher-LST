package cw20

import "github.com/Escher-finance/evm-union-liquid-staking/interchaintest/types/liquidstaking"

type Url string

type Embedded string

type Logo struct {
	LogoUrl      Url      `json:"url"`
	LogoEmbedded Embedded `json:"embedded"`
}

type InstantiateMarketingInfo struct {
	Project     *string `json:"project"`
	Description *string `json:"description"`
	Marketing   *string `json:"marketing"`
	Logo        *Logo   `json:"logo"`
}

type MinterResponse struct {
	Minter string `json:"minter"`
	/// cap is a hard cap on total supply that can be achieved by minting.
	/// Note that this refers to total_supply.
	/// If None, there is unlimited cap.
	Cap *string `json:"cap,omitempty"`
}

type InstantiateMsg struct {
	Name            string                    `json:"name"`
	Symbol          string                    `json:"symbol"`
	Decimals        uint64                    `json:"decimals"`
	InitialBalances []Cw20Coin                `json:"initial_balances"`
	Marketing       *InstantiateMarketingInfo `json:"instantiate_marketing_info,omitempty"`
	Mint            *MinterResponse           `json:"mint,omitempty"`
}

type Cw20Coin struct {
	Amount  string `json:"amount"`
	Address string `json:"address"`
}

type ExecuteMsg_Transfer struct {
	Recipient string                `json:"recipient"`
	Amount    liquidstaking.Uint128 `json:"amount"`
}

// The messages to execute the Liquid Staking contract.
type ExecuteMsg struct {
	Transfer *ExecuteMsg_Transfer `json:"transfer,omitempty"`
}

type QueryMsg_Balance struct {
	Address string `json:"address"`
}

type QueryMsg struct {
	Balance *QueryMsg_Balance `json:"balance,omitempty"`
}

type BalanceResponse struct {
	Balance string `json:"balance"`
}
