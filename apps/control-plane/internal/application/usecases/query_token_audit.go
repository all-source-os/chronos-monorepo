package usecases

import (
	"time"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// QueryTokenAuditUseCase handles querying token usage audit logs.
type QueryTokenAuditUseCase struct {
	auditRepo repositories.AuditRepository
}

// NewQueryTokenAuditUseCase creates a new QueryTokenAuditUseCase.
func NewQueryTokenAuditUseCase(auditRepo repositories.AuditRepository) *QueryTokenAuditUseCase {
	return &QueryTokenAuditUseCase{auditRepo: auditRepo}
}

// Execute queries token usage audit entries with pagination and filtering.
func (uc *QueryTokenAuditUseCase) Execute(req dto.TokenAuditQueryRequest) (*dto.TokenAuditResponse, error) {
	page := req.Page
	if page <= 0 {
		page = 1
	}
	perPage := req.PerPage
	if perPage <= 0 {
		perPage = 20
	}
	if perPage > 100 {
		perPage = 100
	}

	// Determine which audit events to fetch based on filters
	hasTenant := req.TenantID != ""
	hasTimeRange := req.From != "" && req.To != ""

	var allEntries []dto.TokenAuditEntry

	switch {
	case hasTenant && hasTimeRange:
		// Both tenant and time range specified: fetch by tenant, then filter by time
		events, err := uc.auditRepo.FindByTenant(req.TenantID, 10000)
		if err != nil {
			return nil, err
		}
		from, to, err := parseTimeRange(req.From, req.To)
		if err != nil {
			return nil, err
		}
		for _, e := range events {
			if isAPITokenEvent(e.EventType) && !e.Timestamp.Before(from) && !e.Timestamp.After(to) {
				allEntries = append(allEntries, toTokenAuditEntry(e))
			}
		}
	case hasTenant:
		// Tenant filter only
		events, err := uc.auditRepo.FindByTenant(req.TenantID, 10000)
		if err != nil {
			return nil, err
		}
		for _, e := range events {
			if isAPITokenEvent(e.EventType) {
				allEntries = append(allEntries, toTokenAuditEntry(e))
			}
		}
	case hasTimeRange:
		// Time range only
		from, to, err := parseTimeRange(req.From, req.To)
		if err != nil {
			return nil, err
		}
		events, err := uc.auditRepo.FindByTimeRange(from, to)
		if err != nil {
			return nil, err
		}
		for _, e := range events {
			if isAPITokenEvent(e.EventType) {
				allEntries = append(allEntries, toTokenAuditEntry(e))
			}
		}
	default:
		// No filter — get recent API events from error trail as fallback
		events, err := uc.auditRepo.FindByTimeRange(time.Now().Add(-24*time.Hour), time.Now())
		if err != nil {
			// Fall back to FindErrors if time range not supported
			events, err = uc.auditRepo.FindErrors(10000)
			if err != nil {
				return nil, err
			}
		}
		for _, e := range events {
			if isAPITokenEvent(e.EventType) {
				allEntries = append(allEntries, toTokenAuditEntry(e))
			}
		}
	}

	// Paginate
	total := len(allEntries)
	totalPages := (total + perPage - 1) / perPage
	if totalPages == 0 {
		totalPages = 1
	}

	start := (page - 1) * perPage
	end := start + perPage
	if start > total {
		start = total
	}
	if end > total {
		end = total
	}

	return &dto.TokenAuditResponse{
		Entries:    allEntries[start:end],
		Total:      total,
		Page:       page,
		PerPage:    perPage,
		TotalPages: totalPages,
	}, nil
}

// ExecuteSummary returns per-tenant aggregated token usage counts.
func (uc *QueryTokenAuditUseCase) ExecuteSummary(req dto.TokenAuditSummaryRequest) (*dto.TokenAuditSummaryResponse, error) {
	var entries []dto.TokenAuditEntry

	if req.From != "" && req.To != "" {
		from, to, err := parseTimeRange(req.From, req.To)
		if err != nil {
			return nil, err
		}
		events, err := uc.auditRepo.FindByTimeRange(from, to)
		if err != nil {
			return nil, err
		}
		for _, e := range events {
			if isAPITokenEvent(e.EventType) {
				entries = append(entries, toTokenAuditEntry(e))
			}
		}
	} else {
		// Default: last 24 hours
		events, err := uc.auditRepo.FindByTimeRange(time.Now().Add(-24*time.Hour), time.Now())
		if err != nil {
			events, err = uc.auditRepo.FindErrors(10000)
			if err != nil {
				return nil, err
			}
		}
		for _, e := range events {
			if isAPITokenEvent(e.EventType) {
				entries = append(entries, toTokenAuditEntry(e))
			}
		}
	}

	// Aggregate by tenant
	tenantMap := make(map[string]*dto.TenantTokenUsage)
	for _, entry := range entries {
		usage, ok := tenantMap[entry.TenantID]
		if !ok {
			usage = &dto.TenantTokenUsage{
				TenantID: entry.TenantID,
			}
			tenantMap[entry.TenantID] = usage
		}
		usage.TotalCalls++
		if entry.ResponseStatus >= 400 {
			usage.ErrorCount++
		}
		ts := entry.Timestamp.Format(time.RFC3339)
		if usage.LastUsedAt == "" || ts > usage.LastUsedAt {
			usage.LastUsedAt = ts
		}
	}

	tenants := make([]dto.TenantTokenUsage, 0, len(tenantMap))
	for _, usage := range tenantMap {
		tenants = append(tenants, *usage)
	}

	return &dto.TokenAuditSummaryResponse{
		Tenants: tenants,
		Total:   len(tenants),
	}, nil
}

// parseTimeRange parses RFC3339 "from" and "to" strings into time.Time values.
func parseTimeRange(fromStr, toStr string) (from, to time.Time, err error) {
	from, err = time.Parse(time.RFC3339, fromStr)
	if err != nil {
		return time.Time{}, time.Time{}, err
	}
	to, err = time.Parse(time.RFC3339, toStr)
	if err != nil {
		return time.Time{}, time.Time{}, err
	}
	return from, to, nil
}

// isAPITokenEvent checks whether an audit event type represents an API token usage event.
// All audit events logged through the middleware represent API calls made with tokens.
func isAPITokenEvent(eventType string) bool {
	// All audit events are API token usage events — they are logged when
	// authenticated API calls are made. We include everything.
	return eventType != ""
}

// toTokenAuditEntry converts an AuditEvent entity to a TokenAuditEntry DTO.
func toTokenAuditEntry(e *entities.AuditEvent) dto.TokenAuditEntry {
	// Extract key_prefix: first 8 chars of the user ID (API key identifier)
	keyPrefix := e.UserID
	if len(keyPrefix) > 8 {
		keyPrefix = keyPrefix[:8]
	}

	return dto.TokenAuditEntry{
		TenantID:       e.TenantID,
		KeyPrefix:      keyPrefix,
		Endpoint:       e.Path,
		Timestamp:      e.Timestamp,
		ResponseStatus: e.StatusCode,
		IPAddress:      e.IPAddress,
	}
}
