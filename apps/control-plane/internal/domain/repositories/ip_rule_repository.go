package repositories

import "github.com/allsource/control-plane/internal/domain/entities"

// IPRuleRepository defines the interface for IP rule persistence
type IPRuleRepository interface {
	// Save persists an IP rule
	Save(rule *entities.IPRule) error

	// FindByID retrieves an IP rule by ID
	FindByID(id string) (*entities.IPRule, error)

	// FindAll retrieves all IP rules
	FindAll() ([]*entities.IPRule, error)

	// Delete removes an IP rule by ID
	Delete(id string) error

	// Exists checks if an IP rule exists
	Exists(id string) (bool, error)
}
