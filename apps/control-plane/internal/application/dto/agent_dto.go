package dto

// RegisterAgentRequest represents a request to register an AI agent.
type RegisterAgentRequest struct {
	AgentName string `json:"agent_name" binding:"required"`
	AgentType string `json:"agent_type" binding:"required,oneof=mcp sdk cli"`
}

// RegisterAgentResponse represents the complete response after registering an agent.
// Built entirely by the use case — the handler only marshals this to JSON.
type RegisterAgentResponse struct {
	TenantID      string      `json:"tenant_id"`
	APIKey        string      `json:"api_key"`
	Tier          string      `json:"tier"`
	Quotas        AgentQuotas `json:"quotas"`
	WalletAddress string      `json:"wallet_address,omitempty"` // Base USDC wallet for x402 auto-pay (fund this to enable payments)
}

// AgentQuotas represents the quotas assigned to an agent.
type AgentQuotas struct {
	EventsQuota  int `json:"events_quota"`
	QueriesQuota int `json:"queries_quota"`
}
