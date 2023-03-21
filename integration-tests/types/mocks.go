package types

type CounterQueryMsg struct {
	Number *NumberQuery `json:"number,omitempty"`
}

type NumberQuery struct{}

type NumberResponse struct {
	Number uint64 `json:"number"`
}
