package types

import wasmvmtypes "github.com/CosmWasm/wasmvm/types"

// --------------------------- mock-account-factory ----------------------------

type FactoryConfig struct {
	OneCore           string                  `json:"one_core"`
	AllowedSrc        wasmvmtypes.IBCEndpoint `json:"allowed_src"`
	AllowedController string                  `json:"allowed_controller"`
}

type FactoryData struct {
	CodeID         uint64 `json:"code_id"`
	InstantiateMsg []byte `json:"instantiate_msg"`
}

// -------------------------------- mock-sender --------------------------------

type SenderInstantiateMsg struct {
	OneCore string `json:"one_core"`
}

type MockSenderExecuteMsg struct {
	Send   *Send        `json:"send,omitempty"`
	ICS999 *CallbackMsg `json:"ics999,omitempty"`
}

type Send struct {
	ConnectionID string   `json:"connection_id"`
	Actions      []Action `json:"actions"`
}

type SenderQueryMsg struct {
	Outcome  *OutcomeKey    `json:"outcome,omitempty"`
	Outcomes *OutcomesQuery `json:"outcomes,omitempty"`
}

type OutcomeKey struct {
	Dest     wasmvmtypes.IBCEndpoint `json:"dest"`
	Sequence uint64                  `json:"sequence"`
}

type OutcomesQuery struct {
	StartAfter *OutcomeKey `json:"start_after,omitempty"`
	Limit      *uint32     `json:"limit,omitempty"`
}

type OutcomeResponse struct {
	Dest     wasmvmtypes.IBCEndpoint `json:"dest"`
	Sequence uint64                  `json:"sequence"`
	Outcome  PacketOutcome           `json:"outcome"`
}

// ------------------------------- mock-counter --------------------------------

type CounterExecuteMsg struct {
	Increment        *Increment        `json:"increment,omitempty"`
	IncrementButFail *IncrementButFail `json:"increment_but_fail,omitempty"`
}

type Increment struct{}

type IncrementButFail struct{}

type IncrementResult struct {
	NewNumber uint64 `json:"new_number"`
}

type CounterQueryMsg struct {
	Number *NumberQuery `json:"number,omitempty"`
}

type NumberQuery struct{}

type NumberResponse struct {
	Number uint64 `json:"number"`
}

// --------------------------------- mock-dex ----------------------------------

type DexInstantiateMsg struct {
	DenomIn  string `json:"denom_in"`
	DenomOut string `json:"denom_out"`
}

type DexExecuteMsg struct {
	Swap *DexSwap `json:"swap,omitempty"`
}

type DexSwap struct{}

type DexQueryMsg struct {
	Config *DexConfigQuery `json:"Config"`
}

type DexConfigQuery struct{}

type DexConfigResponse DexInstantiateMsg
