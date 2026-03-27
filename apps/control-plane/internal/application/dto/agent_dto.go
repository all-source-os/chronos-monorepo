package dto

// RegisterAgentRequest represents a request to register an AI agent.
type RegisterAgentRequest struct {
	AgentName string `json:"agent_name" binding:"required"`
	AgentType string `json:"agent_type" binding:"required,oneof=mcp sdk cli"`
}

// RegisterAgentResponse represents the response after registering an agent.
type RegisterAgentResponse struct {
	TenantID string            `json:"tenant_id"`
	APIKey   string            `json:"api_key"`
	Tier     string            `json:"tier"`
	Quotas   AgentQuotas       `json:"quotas"`
	CoreURL  string            `json:"core_url"`
	QueryURL string            `json:"query_url"`
	Curl     map[string]string `json:"getting_started,omitempty"`
}

// AgentQuotas represents the quotas assigned to an agent.
type AgentQuotas struct {
	EventsQuota  int `json:"events_quota"`
	QueriesQuota int `json:"queries_quota"`
}
