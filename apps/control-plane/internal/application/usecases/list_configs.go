package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// ListConfigsUseCase handles listing config entries.
type ListConfigsUseCase struct {
	configRepo repositories.ConfigRepository
}

// NewListConfigsUseCase creates a new ListConfigsUseCase.
func NewListConfigsUseCase(configRepo repositories.ConfigRepository) *ListConfigsUseCase {
	return &ListConfigsUseCase{configRepo: configRepo}
}

// Execute lists config entries, optionally filtered by category.
func (uc *ListConfigsUseCase) Execute(category string) ([]*dto.ConfigResponse, error) {
	var result []*entities.ConfigEntry
	var err error

	if category != "" {
		result, err = uc.configRepo.FindByCategory(category)
	} else {
		result, err = uc.configRepo.FindAll()
	}
	if err != nil {
		return nil, err
	}

	responses := make([]*dto.ConfigResponse, len(result))
	for i, entry := range result {
		responses[i] = &dto.ConfigResponse{
			Key:         entry.Key,
			Value:       entry.Value,
			Description: entry.Description,
			Category:    entry.Category,
			UpdatedBy:   entry.UpdatedBy,
			CreatedAt:   entry.CreatedAt,
			UpdatedAt:   entry.UpdatedAt,
		}
	}
	return responses, nil
}
