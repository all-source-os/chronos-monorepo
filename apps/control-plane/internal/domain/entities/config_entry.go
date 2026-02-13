package entities

import (
	"errors"
	"time"
)

// ConfigEntry represents a dynamic configuration entry
type ConfigEntry struct {
	Key         string
	Value       string
	Description string
	Category    string
	UpdatedBy   string
	CreatedAt   time.Time
	UpdatedAt   time.Time
}

// NewConfigEntry creates a new ConfigEntry with validation
func NewConfigEntry(key, value, description, category string) (*ConfigEntry, error) {
	if key == "" {
		return nil, errors.New("config key cannot be empty")
	}
	if value == "" {
		return nil, errors.New("config value cannot be empty")
	}
	if category == "" {
		category = "general"
	}

	now := time.Now()
	return &ConfigEntry{
		Key:         key,
		Value:       value,
		Description: description,
		Category:    category,
		CreatedAt:   now,
		UpdatedAt:   now,
	}, nil
}

// Update modifies the value and description of a config entry
func (c *ConfigEntry) Update(value, description, updatedBy string) {
	if value != "" {
		c.Value = value
	}
	if description != "" {
		c.Description = description
	}
	c.UpdatedBy = updatedBy
	c.UpdatedAt = time.Now()
}
