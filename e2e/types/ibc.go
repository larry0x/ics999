package types

import (
	channeltypes "github.com/cosmos/ibc-go/v6/modules/core/04-channel/types"

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
	Transfer        *TransferAction        `json:"transfer,omitempty"`
	RegisterAccount *RegisterAccountAction `json:"register_account,omitempty"`
	Execute         *wasmvmtypes.WasmMsg   `json:"execute,omitempty"`
	Query           *wasmvmtypes.WasmQuery `json:"query,omitempty"`
}

type TransferAction struct {
	Amount    wasmvmtypes.Coins `json:"amount"`
	Recipient string            `json:"recipient,omitempty"`
}

type RegisterAccountAction struct {
	Salt []byte `json:"salt,omitempty"`
}

type PacketAck struct {
	Result []ActionResult `json:"result,omitempty"`
	Error  string         `json:"error,omitempty"`
}

type ActionResult struct {
	Transfer        *TransferResult        `json:"transfer,omitempty"`
	RegisterAccount *RegisterAccountResult `json:"register_account,omitempty"`
	Execute         *ExecuteResult         `json:"execute,omitempty"`
	Query           *QueryResult           `json:"query,omitempty"`
}

type TransferResult struct {
	Amount    wasmvmtypes.Coins `json:"amount"`
	Recipient string            `json:"recipient"`
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
