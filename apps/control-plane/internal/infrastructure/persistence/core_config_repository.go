package persistence

import (
	"context"
	"fmt"
	"time"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// CoreConfigRepository implements ConfigRepository by delegating to Core via REST.
type CoreConfigRepository struct {
	client clients.CoreClient
}

// NewCoreConfigRepository creates a new CoreConfigRepository.
func NewCoreConfigRepository(client clients.CoreClient) *CoreConfigRepository {
	return &CoreConfigRepository{client: client}
}

// Save persists a config entry to Core.
func (r *CoreConfigRepository) Save(entry *entities.ConfigEntry) error {
	ctx := context.Background()
	return r.client.SetConfig(ctx, clients.SetConfigRequest{
		Key:       entry.Key,
		Value:     entry.Value,
		ChangedBy: entry.UpdatedBy,
	})
}

// FindByKey retrieves a config entry by key from Core.
func (r *CoreConfigRepository) FindByKey(key string) (*entities.ConfigEntry, error) {
	ctx := context.Background()
	resp, err := r.client.GetConfig(ctx, key)
	if err != nil {
		return nil, domain.ErrConfigNotFound
	}
	return coreConfigToEntity(resp), nil
}

// FindAll retrieves all config entries from Core.
func (r *CoreConfigRepository) FindAll() ([]*entities.ConfigEntry, error) {
	ctx := context.Background()
	resp, err := r.client.ListConfigs(ctx)
	if err != nil {
		return nil, err
	}

	result := make([]*entities.ConfigEntry, 0, len(resp.Configs))
	for i := range resp.Configs {
		result = append(result, coreConfigToEntity(&resp.Configs[i]))
	}
	return result, nil
}

// FindByCategory retrieves config entries filtered by category.
// Core stores config as flat key-value pairs without category,
// so we filter client-side — config set is small.
func (r *CoreConfigRepository) FindByCategory(category string) ([]*entities.ConfigEntry, error) {
	all, err := r.FindAll()
	if err != nil {
		return nil, err
	}

	result := make([]*entities.ConfigEntry, 0)
	for _, entry := range all {
		if entry.Category == category {
			result = append(result, entry)
		}
	}
	return result, nil
}

// Delete removes a config entry from Core.
func (r *CoreConfigRepository) Delete(key string) error {
	ctx := context.Background()
	return r.client.DeleteConfig(ctx, key)
}

// Exists checks if a config entry exists in Core.
func (r *CoreConfigRepository) Exists(key string) (bool, error) {
	ctx := context.Background()
	_, err := r.client.GetConfig(ctx, key)
	if err != nil {
		return false, nil //nolint:nilerr // not-found is not an error for Exists
	}
	return true, nil
}

func coreConfigToEntity(resp *clients.ConfigEntryResponse) *entities.ConfigEntry {
	updatedAt, _ := time.Parse(time.RFC3339, resp.UpdatedAt) //nolint:errcheck // best-effort timestamp parsing

	return &entities.ConfigEntry{
		Key:       resp.Key,
		Value:     fmt.Sprintf("%v", resp.Value),
		UpdatedBy: resp.UpdatedBy,
		CreatedAt: updatedAt,
		UpdatedAt: updatedAt,
	}
}
