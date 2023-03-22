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
	QueryRaw        *QueryRawAction        `json:"query_raw,omitempty"`
	QuerySmart      *QuerySmartAction      `json:"query_smart,omitempty"`
	RegisterAccount *RegisterAccountAction `json:"register_account,omitempty"`
	Execute         *wasmvmtypes.CosmosMsg `json:"execute,omitempty"`
}

type TransferAction struct {
	Amount    wasmvmtypes.Coins `json:"amount"`
	Recipient string            `json:"recipient,omitempty"`
}

type QueryRawAction struct {
	Contract string `json:"contract"`
	Key      []byte `json:"key"`
}

type QuerySmartAction struct {
	Contract string `json:"contract"`
	Msg      []byte `json:"msg"`
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
	QueryRaw        *QueryRawResult        `json:"query_raw,omitempty"`
	QuerySmart      *QuerySmartResult      `json:"query_smart,omitempty"`
	RegisterAccount *RegisterAccountResult `json:"register_account,omitempty"`
	Execute         *ExecuteResult         `json:"execute,omitempty"`
}

type TransferResult struct {
	Amount    wasmvmtypes.Coins `json:"amount"`
	Recipient string            `json:"recipient"`
}

type QueryRawResult struct {
	Value []byte `json:"value,omitempty"`
}

type QuerySmartResult struct {
	Response []byte `json:"response"`
}

type RegisterAccountResult struct {
	Address string `json:"address"`
}

type ExecuteResult struct {
	Data []byte `json:"data,omitempty"`
}
