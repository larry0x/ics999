package types

type CoreInstantiateMsg struct {
	AccountCodeID  uint64 `json:"account_code_id"`
	TransferCodeID uint64 `json:"transfer_code_id"`
}
