package dto

import "time"

// CreateAlertRuleRequest represents a request to create an alert rule
type CreateAlertRuleRequest struct {
	Name                string  `json:"name" binding:"required"`
	Metric              string  `json:"metric" binding:"required"`
	Operator            string  `json:"operator" binding:"required"`
	Threshold           float64 `json:"threshold"`
	Duration            string  `json:"duration" binding:"required"`
	NotificationChannel string  `json:"notification_channel" binding:"required"`
}

// UpdateAlertRuleRequest represents a request to update an alert rule
type UpdateAlertRuleRequest struct {
	Name                string   `json:"name"`
	Metric              string   `json:"metric"`
	Operator            string   `json:"operator"`
	Threshold           *float64 `json:"threshold"`
	Duration            string   `json:"duration"`
	NotificationChannel string   `json:"notification_channel"`
	Enabled             *bool    `json:"enabled"`
}

// AlertRuleResponse represents an alert rule in API responses
type AlertRuleResponse struct {
	ID                  string    `json:"id"`
	Name                string    `json:"name"`
	Metric              string    `json:"metric"`
	Operator            string    `json:"operator"`
	Threshold           float64   `json:"threshold"`
	Duration            string    `json:"duration"`
	NotificationChannel string    `json:"notification_channel"`
	Enabled             bool      `json:"enabled"`
	CreatedAt           time.Time `json:"created_at"`
	UpdatedAt           time.Time `json:"updated_at"`
}
