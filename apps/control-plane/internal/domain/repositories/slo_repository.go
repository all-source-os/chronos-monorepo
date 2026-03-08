package repositories

import "github.com/allsource/control-plane/internal/domain/entities"

// SLORepository defines the interface for SLO persistence
type SLORepository interface {
	// Save persists an SLO (insert or update)
	Save(slo *entities.SLO) error

	// FindByID retrieves an SLO by ID
	FindByID(id string) (*entities.SLO, error)

	// FindAll retrieves all SLOs
	FindAll() ([]*entities.SLO, error)

	// Delete removes an SLO by ID
	Delete(id string) error

	// Exists checks if an SLO exists
	Exists(id string) (bool, error)
}
