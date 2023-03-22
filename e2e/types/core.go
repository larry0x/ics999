package types

type CoreInstantiateMsg struct {
	AccountCodeID      uint64 `json:"account_code_id"`
	TransferCodeID     uint64 `json:"transfer_code_id"`
	DefaultTimeoutSecs uint64 `json:"default_timeout_secs"`
}

type CoreExecuteMsg struct {
	Act *Act `json:"act,omitempty"`
}

type Act struct {
	ConnectionID string   `json:"connection_id"`
	Actions      []Action `json:"actions"`
}

type CoreQueryMsg struct {
	Config         *ConfigQuery         `json:"config,omitempty"`
	Account        *AccountQuery        `json:"account,omitempty"`
	Accounts       *AccountsQuery       `json:"accounts,omitempty"`
	ActiveChannel  *ActiveChannelQuery  `json:"active_channel,omitempty"`
	ActiveChannels *ActiveChannelsQuery `json:"active_channels,omitempty"`
}

type ConfigQuery struct{}

type ConfigResponse struct {
	AccountCodeID      uint64 `json:"account_code_id"`
	Transfer           string `json:"transfer"`
	DefaultTimeoutSecs uint64 `json:"default_timeout_secs"`
}

type AccountQuery struct {
	ConnectionID string `json:"connection_id"`
	Controller   string `json:"controller"`
}

type AccountResponse struct {
	ConnectionID string `json:"connection_id"`
	Controller   string `json:"controller"`
	Address      string `json:"address"`
}

type AccountsQuery struct {
	StartAfter []string `json:"start_after,omitempty"`
	Limit      uint32   `json:"limit,omitempty"`
}

type AccountsResponse []AccountResponse

type ActiveChannelQuery struct {
	ConnectionID string `json:"connection_id"`
}

type ActiveChannelResponse struct {
	ConnectionID string `json:"connection_id"`
	ChannelID    string `json:"channel_id"`
}

type ActiveChannelsQuery struct {
	StartAfter string `json:"start_after,omitempty"`
	Limit      uint32 `json:"limit,omitempty"`
}

type ActiveChannelsResponse []ActiveChannelResponse
