package entities

import (
	"errors"
	"time"
)

// SLOWindow represents the time window for SLO measurement
type SLOWindow string

// SLOWindow constants define the supported measurement windows.
const (
	SLOWindow7Days  SLOWindow = "7d"
	SLOWindow30Days SLOWindow = "30d"
)

// SLO represents a Service Level Objective definition
type SLO struct {
	ID            string
	Name          string
	Metric        string
	TargetPercent float64
	Window        SLOWindow
	CreatedAt     time.Time
	UpdatedAt     time.Time
}

// SLOCompliance represents a computed SLO compliance result
type SLOCompliance struct {
	SLO             *SLO
	CurrentPercent  float64
	IsCompliant     bool
	RemainingBudget float64 // percentage points of error budget remaining
	MeasuredAt      time.Time
}

// NewSLO creates a new SLO with validation
func NewSLO(id, name, metric string, targetPercent float64, window SLOWindow) (*SLO, error) {
	if id == "" {
		return nil, errors.New("SLO ID cannot be empty")
	}
	if name == "" {
		return nil, errors.New("SLO name cannot be empty")
	}
	if metric == "" {
		return nil, errors.New("SLO metric cannot be empty")
	}
	if targetPercent <= 0 || targetPercent > 100 {
		return nil, errors.New("SLO target percent must be between 0 and 100 (exclusive/inclusive)")
	}
	if err := ValidateSLOWindow(window); err != nil {
		return nil, err
	}

	now := time.Now()
	return &SLO{
		ID:            id,
		Name:          name,
		Metric:        metric,
		TargetPercent: targetPercent,
		Window:        window,
		CreatedAt:     now,
		UpdatedAt:     now,
	}, nil
}

// ValidateSLOWindow validates an SLO window
func ValidateSLOWindow(window SLOWindow) error {
	switch window {
	case SLOWindow7Days, SLOWindow30Days:
		return nil
	default:
		return errors.New("invalid SLO window: must be 7d or 30d")
	}
}

// CalculateCompliance computes the SLO compliance given a current metric value (percentage).
// currentPercent represents the actual observed compliance percentage.
func (s *SLO) CalculateCompliance(currentPercent float64) *SLOCompliance {
	errorBudgetTotal := 100.0 - s.TargetPercent
	errorBudgetUsed := 100.0 - currentPercent
	remainingBudget := errorBudgetTotal - errorBudgetUsed

	if remainingBudget < 0 {
		remainingBudget = 0
	}

	return &SLOCompliance{
		SLO:             s,
		CurrentPercent:  currentPercent,
		IsCompliant:     currentPercent >= s.TargetPercent,
		RemainingBudget: remainingBudget,
		MeasuredAt:      time.Now(),
	}
}

// Update modifies the SLO fields
func (s *SLO) Update(name, metric string, targetPercent *float64, window SLOWindow) error {
	if name != "" {
		s.Name = name
	}
	if metric != "" {
		s.Metric = metric
	}
	if targetPercent != nil {
		if *targetPercent <= 0 || *targetPercent > 100 {
			return errors.New("SLO target percent must be between 0 and 100 (exclusive/inclusive)")
		}
		s.TargetPercent = *targetPercent
	}
	if window != "" {
		if err := ValidateSLOWindow(window); err != nil {
			return err
		}
		s.Window = window
	}
	s.UpdatedAt = time.Now()
	return nil
}
