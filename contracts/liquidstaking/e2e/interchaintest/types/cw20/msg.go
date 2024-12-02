package cw20

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

type InstantiateMsg struct {
	Name            string                    `json:"name"`
	Symbol          string                    `json:"symbol"`
	Decimals        uint64                    `json:"decimals"`
	InitialBalances []Cw20Coin                `json:"initial_balances"`
	Marketing       *InstantiateMarketingInfo `json:"instantiate_marketing_info"`
}

type Cw20Coin struct {
	Amount  string `json:"amount"`
	Address string `json:"address"`
}

// The messages to execute the Liquid Staking contract.
type ExecuteMsg struct{}

type QueryMsg struct{}
