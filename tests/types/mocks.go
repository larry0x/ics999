package types

// -------------------------------- mock-sender --------------------------------

type SenderInstantiateMsg struct {
	OneCore string `json:"one_core"`
}

type SenderExecuteMsg struct {
	Send           *Send           `json:"send,omitempty"`
	PacketCallback *PacketCallback `json:"packet_callback,omitempty"`
}

type Send struct {
	ConnectionID string   `json:"connection_id"`
	Actions      []Action `json:"actions"`
}

type PacketCallback struct {
	ChannelID string     `json:"channel_id"`
	Sequence  uint64     `json:"sequence"`
	Ack       *PacketAck `json:"act,omitempty"`
}

type SenderQueryMsg struct {
	Outcome *OutcomeQuery `json:"outcome,omitempty"`

	// no idea how to write the Outcomes query in Golang
	// specically the Option<(String, u64)>
	// Golang slices can't have two different types?
	// anyways, we don't use it in tests
}

type OutcomeQuery struct {
	ChannelID string `json:"channel_id"`
	Sequence  uint64 `json:"sequence"`
}

type OutcomeResponse struct {
	ChannelID string `json:"channel_id"`
	Sequence  uint64 `json:"sequence"`
	Outcome   string `json:"outcome"`
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
