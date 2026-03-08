package usecases

import (
	"time"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// MetricsProvider provides current metric values for SLO compliance calculation.
type MetricsProvider interface {
	// GetCurrentMetricPercent returns the current compliance percentage for a metric
	// within the given time window. Returns a value between 0 and 100.
	GetCurrentMetricPercent(metric string, window time.Duration) (float64, error)
}

// ListSLOsUseCase handles listing all SLOs with compliance status.
type ListSLOsUseCase struct {
	sloRepo         repositories.SLORepository
	metricsProvider MetricsProvider
}

// NewListSLOsUseCase creates a new ListSLOsUseCase.
func NewListSLOsUseCase(sloRepo repositories.SLORepository, metricsProvider MetricsProvider) *ListSLOsUseCase {
	return &ListSLOsUseCase{
		sloRepo:         sloRepo,
		metricsProvider: metricsProvider,
	}
}

// Execute lists all SLOs with current compliance calculated from metrics.
func (uc *ListSLOsUseCase) Execute() ([]*dto.SLOComplianceResponse, error) {
	slos, err := uc.sloRepo.FindAll()
	if err != nil {
		return nil, err
	}

	responses := make([]*dto.SLOComplianceResponse, len(slos))
	for i, slo := range slos {
		sloResp := toSLOResponse(slo)

		// Calculate compliance from metrics provider
		var currentPercent float64
		if uc.metricsProvider != nil {
			window := windowToDuration(slo.Window)
			currentPercent, _ = uc.metricsProvider.GetCurrentMetricPercent(slo.Metric, window) //nolint:errcheck
		} else {
			// Default: assume 100% compliance when no metrics provider
			currentPercent = 100.0
		}

		compliance := slo.CalculateCompliance(currentPercent)

		responses[i] = &dto.SLOComplianceResponse{
			SLOResponse:     *sloResp,
			CurrentPercent:  compliance.CurrentPercent,
			IsCompliant:     compliance.IsCompliant,
			RemainingBudget: compliance.RemainingBudget,
			MeasuredAt:      compliance.MeasuredAt,
		}
	}
	return responses, nil
}

// windowToDuration converts an SLO window to a time.Duration.
func windowToDuration(window entities.SLOWindow) time.Duration {
	switch string(window) {
	case "7d":
		return 7 * 24 * time.Hour
	case "30d":
		return 30 * 24 * time.Hour
	default:
		return 30 * 24 * time.Hour
	}
}
