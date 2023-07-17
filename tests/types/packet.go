package types

import (
	sdk "github.com/cosmos/cosmos-sdk/types"

	channeltypes "github.com/cosmos/ibc-go/v4/modules/core/04-channel/types"

	wasmvmtypes "github.com/CosmWasm/wasmvm/types"
)

// ---------------------------------- channel ----------------------------------

const (
	Order   = channeltypes.UNORDERED
	Version = "ics999-1"
)

// ---------------------------------- packet -----------------------------------

type PacketData struct {
	Sender  string   `json:"sender"`
	Actions []Action `json:"actions"`
	Traces  []Trace  `json:"traces"`
}

type Action struct {
	Transfer        *TransferAction        `json:"transfer,omitempty"`
	RegisterAccount *RegisterAccountAction `json:"register_account,omitempty"`
	Execute         []byte                 `json:"execute,omitempty"`
	Query           []byte                 `json:"query,omitempty"`
}

type TransferAction struct {
	Denom     string  `json:"denom"`
	Amount    sdk.Int `json:"amount"`
	Recipient string  `json:"recipient,omitempty"`
}

type RegisterAccountAction struct {
	Default       *RegisterAccountDefault       `json:"default,omitempty"`
	CustomFactory *RegisterAccountCustomFactory `json:"custom_factory,omitempty"`
}

type RegisterAccountDefault struct {
	Salt []byte `json:"salt,omitempty"`
}

type RegisterAccountCustomFactory struct {
	Address string `json:"address,omitempty"`
	Data    []byte `json:"data,omitempty"`
}

// ------------------------------------ ack ------------------------------------

type PacketAck struct {
	Success []ActionResult `json:"success,omitempty"`
	Failed  string         `json:"failed,omitempty"`
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

// ----------------------------------- trace -----------------------------------

type Trace struct {
	Denom     string                    `json:"denom"`
	BaseDenom string                    `json:"base_denom"`
	Path      []wasmvmtypes.IBCEndpoint `json:"path"`
}

type TraceItem struct {
	BaseDenom string                    `json:"base_denom"`
	Path      []wasmvmtypes.IBCEndpoint `json:"path"`
}

// --------------------------- third party: factory ----------------------------

type FactoryExecuteMsg struct {
	ICS999 *FactoryMsg `json:"ics999,omitempty"`
}

type FactoryMsg struct {
	Src        wasmvmtypes.IBCEndpoint `json:"src"`
	Controller string                  `json:"controller"`
	Data       []byte                  `json:"data,omitempty"`
}

type FactoryResponse struct {
	Host string `json:"host"`
}

// ---------------------------- third party: sender ----------------------------

type SenderExecuteMsg struct {
	ICS999 *CallbackMsg `json:"ics999,omitempty"`
}

type CallbackMsg struct {
	Dest     wasmvmtypes.IBCEndpoint `json:"dest"`
	Sequence uint64                  `json:"sequence"`
	Outcome  PacketOutcome           `json:"outcome"`
}

type PacketOutcome struct {
	Success []ActionResult `json:"success,omitempty"`
	Failed  string         `json:"failed,omitempty"`
	Timeout *Timeout       `json:"timeout,omitempty"`
}

type Timeout struct{}
