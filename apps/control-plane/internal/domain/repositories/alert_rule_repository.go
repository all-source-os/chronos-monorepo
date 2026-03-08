package repositories

import "github.com/allsource/control-plane/internal/domain/entities"

// AlertRuleRepository defines the interface for alert rule persistence
type AlertRuleRepository interface {
	// Save persists an alert rule (insert or update)
	Save(rule *entities.AlertRule) error

	// FindByID retrieves an alert rule by ID
	FindByID(id string) (*entities.AlertRule, error)

	// FindAll retrieves all alert rules
	FindAll() ([]*entities.AlertRule, error)

	// Delete removes an alert rule by ID
	Delete(id string) error

	// Exists checks if an alert rule exists
	Exists(id string) (bool, error)
}
