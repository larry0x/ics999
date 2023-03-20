package types

import (
	channeltypes "github.com/cosmos/ibc-go/v6/modules/core/04-channel/types"

	wasmvmtypes "github.com/CosmWasm/wasmvm/types"
)

const (
	Order   = channeltypes.UNORDERED
	Version = "ics999-1"
)

type Action struct {
	Transfer        *TransferAction        `json:"transfer,omitempty"`
	RegisterAccount *RegisterAccountAction `json:"register_account,omitempty"`
	Execute         *wasmvmtypes.WasmMsg   `json:"execute,omitempty"`
}

type TransferAction struct {
	Amount    wasmvmtypes.Coins `json:"amount"`
	Recipient string            `json:"recipient,omitempty"`
}

type RegisterAccountAction struct {
	Salt []byte `json:"salt,omitempty"`
}
