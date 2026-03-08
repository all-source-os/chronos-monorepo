package dto

import "time"

// TokenAuditQueryRequest represents a request to query token usage audit logs.
type TokenAuditQueryRequest struct {
	TenantID string `form:"tenant_id"`
	From     string `form:"from"`
	To       string `form:"to"`
	Page     int    `form:"page"`
	PerPage  int    `form:"per_page"`
}

// TokenAuditEntry represents a single token usage log entry.
type TokenAuditEntry struct {
	TenantID       string    `json:"tenant_id"`
	KeyPrefix      string    `json:"key_prefix"`
	Endpoint       string    `json:"endpoint"`
	Timestamp      time.Time `json:"timestamp"`
	ResponseStatus int       `json:"response_status"`
	IPAddress      string    `json:"ip_address"`
}

// TokenAuditResponse is the paginated response for token audit queries.
type TokenAuditResponse struct {
	Entries    []TokenAuditEntry `json:"entries"`
	Total      int               `json:"total"`
	Page       int               `json:"page"`
	PerPage    int               `json:"per_page"`
	TotalPages int               `json:"total_pages"`
}

// TokenAuditSummaryRequest represents a request for the aggregation summary.
type TokenAuditSummaryRequest struct {
	From string `form:"from"`
	To   string `form:"to"`
}

// TenantTokenUsage represents per-tenant token usage counts.
type TenantTokenUsage struct {
	TenantID   string `json:"tenant_id"`
	TotalCalls int    `json:"total_calls"`
	ErrorCount int    `json:"error_count"`
	LastUsedAt string `json:"last_used_at,omitempty"`
}

// TokenAuditSummaryResponse is the response for the aggregation summary endpoint.
type TokenAuditSummaryResponse struct {
	Tenants []TenantTokenUsage `json:"tenants"`
	Total   int                `json:"total"`
}
