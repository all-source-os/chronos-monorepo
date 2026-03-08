package entities

import (
	"errors"
	"time"
)

// AlertOperator represents a comparison operator for alert rules
type AlertOperator string

// AlertOperator constants define the supported comparison operators.
const (
	OperatorGreaterThan AlertOperator = ">"
	OperatorLessThan    AlertOperator = "<"
	OperatorEqual       AlertOperator = "=="
)

// AlertRule represents a rule that triggers an alert when a metric crosses a threshold
type AlertRule struct {
	ID                  string
	Name                string
	Metric              string
	Operator            AlertOperator
	Threshold           float64
	Duration            string // e.g., "5m", "1h"
	NotificationChannel string
	Enabled             bool
	CreatedAt           time.Time
	UpdatedAt           time.Time
}

// NewAlertRule creates a new AlertRule with validation
func NewAlertRule(id, name, metric string, operator AlertOperator, threshold float64, duration, notificationChannel string) (*AlertRule, error) {
	if id == "" {
		return nil, errors.New("alert rule ID cannot be empty")
	}
	if name == "" {
		return nil, errors.New("alert rule name cannot be empty")
	}
	if metric == "" {
		return nil, errors.New("alert rule metric cannot be empty")
	}
	if err := ValidateAlertOperator(operator); err != nil {
		return nil, err
	}
	if duration == "" {
		return nil, errors.New("alert rule duration cannot be empty")
	}
	if notificationChannel == "" {
		return nil, errors.New("notification channel cannot be empty")
	}

	now := time.Now()
	return &AlertRule{
		ID:                  id,
		Name:                name,
		Metric:              metric,
		Operator:            operator,
		Threshold:           threshold,
		Duration:            duration,
		NotificationChannel: notificationChannel,
		Enabled:             true,
		CreatedAt:           now,
		UpdatedAt:           now,
	}, nil
}

// ValidateAlertOperator validates an alert operator
func ValidateAlertOperator(op AlertOperator) error {
	switch op {
	case OperatorGreaterThan, OperatorLessThan, OperatorEqual:
		return nil
	default:
		return errors.New("invalid alert operator: must be >, <, or ==")
	}
}

// Update modifies the alert rule fields
func (a *AlertRule) Update(name, metric string, operator AlertOperator, threshold *float64, duration, notificationChannel string, enabled *bool) error {
	if name != "" {
		a.Name = name
	}
	if metric != "" {
		a.Metric = metric
	}
	if operator != "" {
		if err := ValidateAlertOperator(operator); err != nil {
			return err
		}
		a.Operator = operator
	}
	if threshold != nil {
		a.Threshold = *threshold
	}
	if duration != "" {
		a.Duration = duration
	}
	if notificationChannel != "" {
		a.NotificationChannel = notificationChannel
	}
	if enabled != nil {
		a.Enabled = *enabled
	}
	a.UpdatedAt = time.Now()
	return nil
}

// Enable enables the alert rule
func (a *AlertRule) Enable() {
	a.Enabled = true
	a.UpdatedAt = time.Now()
}

// Disable disables the alert rule
func (a *AlertRule) Disable() {
	a.Enabled = false
	a.UpdatedAt = time.Now()
}
