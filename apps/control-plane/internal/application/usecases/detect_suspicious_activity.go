package usecases

import (
	"fmt"
	"time"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// Detection thresholds (hardcoded v1).
const (
	// FailedAuthThreshold is the maximum number of failed auth attempts per tenant in 1 hour.
	FailedAuthThreshold = 100
	// FailedAuthWindow is the time window for failed auth detection.
	FailedAuthWindow = 1 * time.Hour

	// APIKeyCreationThreshold is the maximum number of API keys created per tenant in 1 hour.
	APIKeyCreationThreshold = 10
	// APIKeyCreationWindow is the time window for API key creation detection.
	APIKeyCreationWindow = 1 * time.Hour

	// UniqueIPThreshold is the maximum number of unique IPs per tenant in 5 minutes.
	UniqueIPThreshold = 20
	// UniqueIPWindow is the time window for unique IP detection.
	UniqueIPWindow = 5 * time.Minute
)

// DetectSuspiciousActivityUseCase analyzes audit logs on-demand to detect suspicious activity.
type DetectSuspiciousActivityUseCase struct {
	auditRepo repositories.AuditRepository
}

// NewDetectSuspiciousActivityUseCase creates a new DetectSuspiciousActivityUseCase.
func NewDetectSuspiciousActivityUseCase(auditRepo repositories.AuditRepository) *DetectSuspiciousActivityUseCase {
	return &DetectSuspiciousActivityUseCase{auditRepo: auditRepo}
}

// Execute analyzes recent audit events and returns any detected suspicious activity alerts.
func (uc *DetectSuspiciousActivityUseCase) Execute() (*dto.SuspiciousActivityResponse, error) {
	now := time.Now()

	// Fetch events for the longest window we need (1 hour)
	events, err := uc.auditRepo.FindByTimeRange(now.Add(-FailedAuthWindow), now)
	if err != nil {
		return nil, fmt.Errorf("failed to query audit events: %w", err)
	}

	alerts := make([]dto.SuspiciousActivityAlert, 0, 10) //nolint:prealloc // size unknown upfront

	// Rule 1: >100 failed auth attempts per tenant in 1 hour
	failedAuthAlerts := uc.detectFailedAuth(events, now)
	alerts = append(alerts, failedAuthAlerts...)

	// Rule 2: >10 API keys created per tenant in 1 hour
	apiKeyAlerts := uc.detectExcessiveAPIKeyCreation(events, now)
	alerts = append(alerts, apiKeyAlerts...)

	// Rule 3: >20 unique IPs for same tenant in 5 minutes
	uniqueIPAlerts := uc.detectUniqueIPBurst(events, now)
	alerts = append(alerts, uniqueIPAlerts...)

	return &dto.SuspiciousActivityResponse{
		Alerts: alerts,
		Total:  len(alerts),
	}, nil
}

// detectFailedAuth detects tenants with more than FailedAuthThreshold failed auth attempts within FailedAuthWindow.
func (uc *DetectSuspiciousActivityUseCase) detectFailedAuth(events []*entities.AuditEvent, now time.Time) []dto.SuspiciousActivityAlert {
	windowStart := now.Add(-FailedAuthWindow)

	// Count failed auth per tenant
	tenantFailedAuth := make(map[string]int)
	for _, e := range events {
		if !e.Timestamp.Before(windowStart) && isFailedAuth(e) {
			tenantFailedAuth[e.TenantID]++
		}
	}

	var alerts []dto.SuspiciousActivityAlert
	for tenantID, count := range tenantFailedAuth {
		if count > FailedAuthThreshold {
			alerts = append(alerts, dto.SuspiciousActivityAlert{
				Type:        "excessive_failed_auth",
				TenantID:    tenantID,
				Description: fmt.Sprintf("Tenant %s had %d failed authentication attempts in the last hour (threshold: %d)", tenantID, count, FailedAuthThreshold),
				Severity:    "high",
				Timestamp:   now,
				Details: map[string]any{
					"failed_count": count,
					"threshold":    FailedAuthThreshold,
					"window":       FailedAuthWindow.String(),
				},
			})
		}
	}

	return alerts
}

// detectExcessiveAPIKeyCreation detects tenants with more than APIKeyCreationThreshold API keys created within APIKeyCreationWindow.
func (uc *DetectSuspiciousActivityUseCase) detectExcessiveAPIKeyCreation(events []*entities.AuditEvent, now time.Time) []dto.SuspiciousActivityAlert {
	windowStart := now.Add(-APIKeyCreationWindow)

	// Count API key creation events per tenant
	tenantKeyCreation := make(map[string]int)
	for _, e := range events {
		if !e.Timestamp.Before(windowStart) && isAPIKeyCreation(e) {
			tenantKeyCreation[e.TenantID]++
		}
	}

	var alerts []dto.SuspiciousActivityAlert
	for tenantID, count := range tenantKeyCreation {
		if count > APIKeyCreationThreshold {
			alerts = append(alerts, dto.SuspiciousActivityAlert{
				Type:        "excessive_api_key_creation",
				TenantID:    tenantID,
				Description: fmt.Sprintf("Tenant %s created %d API keys in the last hour (threshold: %d)", tenantID, count, APIKeyCreationThreshold),
				Severity:    "medium",
				Timestamp:   now,
				Details: map[string]any{
					"keys_created": count,
					"threshold":    APIKeyCreationThreshold,
					"window":       APIKeyCreationWindow.String(),
				},
			})
		}
	}

	return alerts
}

// detectUniqueIPBurst detects tenants with more than UniqueIPThreshold unique IPs within UniqueIPWindow.
func (uc *DetectSuspiciousActivityUseCase) detectUniqueIPBurst(events []*entities.AuditEvent, now time.Time) []dto.SuspiciousActivityAlert {
	windowStart := now.Add(-UniqueIPWindow)

	// Collect unique IPs per tenant in the 5-minute window
	tenantIPs := make(map[string]map[string]struct{})
	for _, e := range events {
		if !e.Timestamp.Before(windowStart) && e.IPAddress != "" {
			if _, ok := tenantIPs[e.TenantID]; !ok {
				tenantIPs[e.TenantID] = make(map[string]struct{})
			}
			tenantIPs[e.TenantID][e.IPAddress] = struct{}{}
		}
	}

	var alerts []dto.SuspiciousActivityAlert
	for tenantID, ips := range tenantIPs {
		count := len(ips)
		if count > UniqueIPThreshold {
			// Collect sample IPs for the details (up to 10)
			sampleIPs := make([]string, 0, 10)
			for ip := range ips {
				sampleIPs = append(sampleIPs, ip)
				if len(sampleIPs) >= 10 {
					break
				}
			}

			alerts = append(alerts, dto.SuspiciousActivityAlert{
				Type:        "unique_ip_burst",
				TenantID:    tenantID,
				Description: fmt.Sprintf("Tenant %s had requests from %d unique IPs in the last 5 minutes (threshold: %d)", tenantID, count, UniqueIPThreshold),
				Severity:    "high",
				Timestamp:   now,
				Details: map[string]any{
					"unique_ip_count": count,
					"threshold":       UniqueIPThreshold,
					"window":          UniqueIPWindow.String(),
					"sample_ips":      sampleIPs,
				},
			})
		}
	}

	return alerts
}

// isFailedAuth checks whether an audit event represents a failed authentication attempt.
func isFailedAuth(e *entities.AuditEvent) bool {
	return e.StatusCode == 401 || e.StatusCode == 403
}

// isAPIKeyCreation checks whether an audit event represents an API key creation.
func isAPIKeyCreation(e *entities.AuditEvent) bool {
	return e.EventType == "api_key_create" || e.EventType == "token_create" ||
		(e.Action == "create" && e.Resource == "api_key")
}
