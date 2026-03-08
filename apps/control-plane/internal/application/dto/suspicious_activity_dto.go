package dto

import "time"

// SuspiciousActivityAlert represents a single suspicious activity alert.
type SuspiciousActivityAlert struct {
	Type        string         `json:"type"`
	TenantID    string         `json:"tenant_id"`
	Description string         `json:"description"`
	Severity    string         `json:"severity"`
	Timestamp   time.Time      `json:"timestamp"`
	Details     map[string]any `json:"details"`
}

// SuspiciousActivityResponse is the response for the suspicious activity endpoint.
type SuspiciousActivityResponse struct {
	Alerts []SuspiciousActivityAlert `json:"alerts"`
	Total  int                       `json:"total"`
}
