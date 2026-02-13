package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// GetConfigUseCase handles retrieving a config entry by key.
type GetConfigUseCase struct {
	configRepo repositories.ConfigRepository
}

// NewGetConfigUseCase creates a new GetConfigUseCase.
func NewGetConfigUseCase(configRepo repositories.ConfigRepository) *GetConfigUseCase {
	return &GetConfigUseCase{configRepo: configRepo}
}

// Execute retrieves a config entry by key.
func (uc *GetConfigUseCase) Execute(key string) (*dto.ConfigResponse, error) {
	entry, err := uc.configRepo.FindByKey(key)
	if err != nil {
		return nil, err
	}

	return &dto.ConfigResponse{
		Key:         entry.Key,
		Value:       entry.Value,
		Description: entry.Description,
		Category:    entry.Category,
		UpdatedBy:   entry.UpdatedBy,
		CreatedAt:   entry.CreatedAt,
		UpdatedAt:   entry.UpdatedAt,
	}, nil
}
