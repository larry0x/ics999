package types

import wasmvmtypes "github.com/CosmWasm/wasmvm/types"

type CoreConfig struct {
	DefaultAccountCodeID uint64 `json:"default_account_code_id"`
	DefaultTimeoutSecs   uint64 `json:"default_timeout_secs"`
}

type CoreExecuteMsg struct {
	Act    *Act    `json:"act,omitempty"`
	Handle *Handle `json:"handle,omitempty"`
}

type Act struct {
	ConnectionID string                  `json:"connection_id"`
	Actions      []Action                `json:"actions"`
	Timeout      *wasmvmtypes.IBCTimeout `json:"timeout,omitempty"`
}

type Handle struct {
	CounterpartyEndpoint wasmvmtypes.IBCEndpoint `json:"counterparty_endpoint"`
	Endpoint             wasmvmtypes.IBCEndpoint `json:"endpoint"`
	Controller           string                  `json:"controller"`
	Actions              []Action                `json:"actions"`
	Traces               []Trace                 `json:"traces"`
}

type CoreQueryMsg struct {
	Config         *ConfigQuery         `json:"config,omitempty"`
	DenomHash      *DenomHashQuery      `json:"denom_hash,omitempty"`
	DenomTrace     *DenomTraceQuery     `json:"denom_trace,omitempty"`
	DenomTraces    *DenomTracesQuery    `json:"denom_traces,omitempty"`
	Account        *AccountKey          `json:"account,omitempty"`
	Accounts       *AccountsQuery       `json:"accounts,omitempty"`
	ActiveChannel  *ActiveChannelQuery  `json:"active_channel,omitempty"`
	ActiveChannels *ActiveChannelsQuery `json:"active_channels,omitempty"`
}

type ConfigQuery struct{}

type DenomHashQuery struct {
	Trace TraceItem `json:"trace"`
}

type DenomHashResponse struct {
	Hash string `json:"hash"` // hex-encoded string
}

type DenomTraceQuery struct {
	Denom string `json:"denom"`
}

type DenomTraceResponse Trace

type DenomTracesQuery struct {
	StartAfter *string `json:"start_after,omitempty"`
	Limit      *uint32 `json:"limit,omitempty"`
}

type DenomTracesResponse []Trace

type AccountKey struct {
	Src        wasmvmtypes.IBCEndpoint `json:"src"`
	Controller string                  `json:"controller"`
}

type AccountResponse struct {
	Src        wasmvmtypes.IBCEndpoint `json:"src"`
	Controller string                  `json:"controller"`
	Address    string                  `json:"address"`
}

type AccountsQuery struct {
	StartAfter *AccountKey `json:"start_after,omitempty"`
	Limit      *uint32     `json:"limit,omitempty"`
}

type AccountsResponse []AccountResponse

type ActiveChannelQuery struct {
	ConnectionID string `json:"connection_id"`
}

type ActiveChannelResponse struct {
	ConnectionID string                  `json:"connection_id"`
	Endpoint     wasmvmtypes.IBCEndpoint `json:"endpoint"`
}

type ActiveChannelsQuery struct {
	StartAfter *string `json:"start_after,omitempty"`
	Limit      *uint32 `json:"limit,omitempty"`
}

type ActiveChannelsResponse []ActiveChannelResponse
