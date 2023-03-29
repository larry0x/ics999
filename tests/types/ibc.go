package types

import (
	sdk "github.com/cosmos/cosmos-sdk/types"

	channeltypes "github.com/cosmos/ibc-go/v4/modules/core/04-channel/types"

	wasmvmtypes "github.com/CosmWasm/wasmvm/types"
)

const (
	Order   = channeltypes.UNORDERED
	Version = "ics999-1"
)

type PacketData struct {
	Sender  string   `json:"sender"`
	Actions []Action `json:"actions"`
}

type Action struct {
	Transfer        *TransferAction           `json:"transfer,omitempty"`
	RegisterAccount *RegisterAccountAction    `json:"register_account,omitempty"`
	Execute         *wasmvmtypes.CosmosMsg    `json:"execute,omitempty"`
	Query           *wasmvmtypes.QueryRequest `json:"query,omitempty"`
}

type TransferAction struct {
	Denom     string  `json:"denom"`
	Amount    sdk.Int `json:"amount"`
	Recipient string  `json:"recipient,omitempty"`
}

type RegisterAccountAction struct {
	Salt []byte `json:"salt,omitempty"`
}

type PacketAck struct {
	Results []ActionResult `json:"results,omitempty"`
	Error   string         `json:"error,omitempty"`
}

type ActionResult struct {
	Transfer        *TransferResult        `json:"transfer,omitempty"`
	RegisterAccount *RegisterAccountResult `json:"register_account,omitempty"`
	Execute         *ExecuteResult         `json:"execute,omitempty"`
	Query           *QueryResult           `json:"query,omitempty"`
}

type TransferResult struct {
	Denom     string `json:"denom"`
	NewToken  bool   `json:"new_token"`
	Recipient string `json:"recipient"`
}

type RegisterAccountResult struct {
	Address string `json:"address"`
}

type ExecuteResult struct {
	Data []byte `json:"data,omitempty"`
}

type QueryResult struct {
	Response []byte `json:"response"`
}
