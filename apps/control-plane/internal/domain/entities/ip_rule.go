package entities

import (
	"errors"
	"net"
	"time"
)

// IPRuleType represents whether an IP rule is an allow or block rule
type IPRuleType string

// IPRuleType constants define the supported IP rule types.
const (
	IPRuleTypeAllow IPRuleType = "allow"
	IPRuleTypeBlock IPRuleType = "block"
)

// IPRule represents an IP allowlist/blocklist entry
type IPRule struct {
	ID          string
	CIDR        string
	Type        IPRuleType
	Description string
	CreatedAt   time.Time
	CreatedBy   string
}

// NewIPRule creates a new IPRule with validation
func NewIPRule(id, cidr string, ruleType IPRuleType, description, createdBy string) (*IPRule, error) {
	if id == "" {
		return nil, errors.New("IP rule ID cannot be empty")
	}
	if err := ValidateCIDR(cidr); err != nil {
		return nil, err
	}
	if err := ValidateIPRuleType(ruleType); err != nil {
		return nil, err
	}

	return &IPRule{
		ID:          id,
		CIDR:        cidr,
		Type:        ruleType,
		Description: description,
		CreatedAt:   time.Now(),
		CreatedBy:   createdBy,
	}, nil
}

// ValidateCIDR validates that the given string is a valid CIDR notation
func ValidateCIDR(cidr string) error {
	if cidr == "" {
		return errors.New("CIDR cannot be empty")
	}
	_, _, err := net.ParseCIDR(cidr)
	if err != nil {
		// Also accept a plain IP address (treat as /32 or /128)
		ip := net.ParseIP(cidr)
		if ip == nil {
			return errors.New("invalid CIDR notation: " + cidr)
		}
	}
	return nil
}

// ValidateIPRuleType validates an IP rule type
func ValidateIPRuleType(ruleType IPRuleType) error {
	switch ruleType {
	case IPRuleTypeAllow, IPRuleTypeBlock:
		return nil
	default:
		return errors.New("invalid IP rule type: must be 'allow' or 'block'")
	}
}
