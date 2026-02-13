package repositories

import "github.com/allsource/control-plane/internal/domain/entities"

// ConfigRepository defines the interface for configuration persistence
type ConfigRepository interface {
	// Save persists a config entry (insert or update)
	Save(entry *entities.ConfigEntry) error

	// FindByKey retrieves a config entry by key
	FindByKey(key string) (*entities.ConfigEntry, error)

	// FindAll retrieves all config entries
	FindAll() ([]*entities.ConfigEntry, error)

	// FindByCategory retrieves config entries by category
	FindByCategory(category string) ([]*entities.ConfigEntry, error)

	// Delete removes a config entry by key
	Delete(key string) error

	// Exists checks if a config entry exists
	Exists(key string) (bool, error)
}
