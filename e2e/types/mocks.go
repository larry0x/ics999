package types

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
