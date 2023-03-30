package types

type OwnableQueryMsg struct {
	Ownership *OwnershipQuery `json:"ownership,omitempty"`
}

type OwnershipQuery struct{}

type OwnershipResponse struct {
	Owner         string      `json:"owner,omitempty"`
	PendingOwner  string      `json:"pending_owner,omitempty"`
	PendingExpiry *Expiration `json:"pending_expiry,omitempty"`
}

type Expiration struct {
	AtHeight uint64 `json:"at_height,omitempty"`
	AtTime   uint64 `json:"at_time,string,omitempty"`
	Never    *Never `json:"never,omitempty"`
}

type Never struct{}
