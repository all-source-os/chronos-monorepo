package entities

import "time"

// OperationType represents the type of an operation
type OperationType string

// OperationType constants
const (
	OperationSnapshot   OperationType = "snapshot"
	OperationCompaction OperationType = "compaction"
	OperationReplay     OperationType = "replay"
	OperationBackup     OperationType = "backup"
)

// OperationStatus represents the status of an operation
type OperationStatus string

// OperationStatus constants
const (
	OperationPending   OperationStatus = "pending"
	OperationRunning   OperationStatus = "running"
	OperationCompleted OperationStatus = "completed"
	OperationFailed    OperationStatus = "failed"
	OperationCancelled OperationStatus = "cancelled"
)

// Operation represents a long-running operation in the system
type Operation struct {
	ID          string
	Type        OperationType
	Status      OperationStatus
	TenantID    string
	Parameters  map[string]any
	Result      map[string]any
	InitiatedBy string
	StartedAt   *time.Time
	CompletedAt *time.Time
	Error       string
	CreatedAt   time.Time
}
