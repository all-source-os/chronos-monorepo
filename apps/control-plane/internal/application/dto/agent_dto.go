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

// RegisterTrialAgentRequest represents a request to mint an anonymous-trial
// API key without prior authentication. Closes Gap 1 of
// docs/proposals/AGENT_DRIVEN_PRIME_ONBOARDING.md so an agent walking a
// human through the install protocol can call this endpoint and skip the
// /connect signup round-trip.
//
// ClientFingerprint is optional and opaque to the server — typically a hash
// of UA + IP set by the caller for self-debugging. The endpoint itself
// rate-limits per source IP regardless.
type RegisterTrialAgentRequest struct {
	AgentName         string `json:"agent_name,omitempty"`
	ClientFingerprint string `json:"client_fingerprint,omitempty"`
}

// RegisterTrialAgentResponse mirrors the proposal's wire shape. The
// claim_token + claim_url let a human later attach the trial's tenant
// to their real account via /connect?claim=<token>.
type RegisterTrialAgentResponse struct {
	TenantID   string      `json:"tenant_id"`
	APIKey     string      `json:"api_key"`
	Tier       string      `json:"tier"` // always "trial" for this endpoint
	Quotas     AgentQuotas `json:"quotas"`
	ExpiresAt  string      `json:"expires_at"`  // RFC3339
	ClaimToken string      `json:"claim_token"` // opaque, single-use
	ClaimURL   string      `json:"claim_url"`   // pre-built /connect?claim=<token>
}
