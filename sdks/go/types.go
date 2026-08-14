package allsource

import "fmt"

// Event represents an event in the AllSource event store.
type Event struct {
	ID        string         `json:"id"`
	EntityID  string         `json:"entity_id"`
	EventType string         `json:"event_type"`
	Payload   map[string]any `json:"payload"`
	Metadata  map[string]any `json:"metadata"`
	TenantID  string         `json:"tenant_id,omitempty"`
	Timestamp string         `json:"timestamp"`
	Version   int            `json:"version"`
}

// EventList is a list of events with a count.
type EventList struct {
	Events []Event `json:"events"`
	Count  int     `json:"count"`
}

// Projection represents a projection in the AllSource event store.
type Projection struct {
	ID           string         `json:"id"`
	Name         string         `json:"name"`
	Version      int            `json:"version"`
	Status       string         `json:"status"`
	InitialState map[string]any `json:"initial_state"`
	Definition   string         `json:"definition"`
	CreatedAt    string         `json:"created_at"`
	UpdatedAt    string         `json:"updated_at"`
}

// ProjectionReplayEventType is one event-type bucket from replay analysis.
type ProjectionReplayEventType struct {
	EventType string  `json:"event_type"`
	Count     int     `json:"count"`
	Share     float64 `json:"share"`
}

// ProjectionReplayEntity is one frequently affected entity from the analysis sample.
type ProjectionReplayEntity struct {
	EntityID   string `json:"entity_id"`
	EventCount int    `json:"event_count"`
}

// ProjectionReplayCheck is a server-asserted replay invariant.
type ProjectionReplayCheck struct {
	Key    string `json:"key"`
	Label  string `json:"label"`
	Status string `json:"status"`
	Detail string `json:"detail"`
}

// ProjectionReplayAnalysis describes replay impact without mutating state.
type ProjectionReplayAnalysis struct {
	ProjectionName        string                      `json:"projection_name"`
	ProjectionTitle       string                      `json:"projection_title"`
	ProjectionKind        string                      `json:"projection_kind"`
	ProjectionStatus      string                      `json:"projection_status"`
	CurrentEntityCount    int                         `json:"current_entity_count"`
	TotalEvents           int                         `json:"total_events"`
	SampledEvents         int                         `json:"sampled_events"`
	AnalysisScope         string                      `json:"analysis_scope"`
	EventTypeDistribution []ProjectionReplayEventType `json:"event_type_distribution"`
	SampledEntityCount    int                         `json:"sampled_entity_count"`
	SampledEntities       []ProjectionReplayEntity    `json:"sampled_entities"`
	FirstEventAt          *string                     `json:"first_event_at"`
	LastEventAt           *string                     `json:"last_event_at"`
	AnalyzedAt            string                      `json:"analyzed_at"`
	ReadyToReplay         bool                        `json:"ready_to_replay"`
	Checks                []ProjectionReplayCheck     `json:"checks"`
	Warnings              []string                    `json:"warnings"`
}

// ProjectionReplayRun is one tenant-scoped projection rebuild.
type ProjectionReplayRun struct {
	ReplayID           string  `json:"replay_id"`
	ProjectionName     string  `json:"projection_name"`
	Status             string  `json:"status"`
	StartedAt          string  `json:"started_at"`
	UpdatedAt          string  `json:"updated_at"`
	CompletedAt        *string `json:"completed_at"`
	TotalEvents        int     `json:"total_events"`
	ProcessedEvents    int     `json:"processed_events"`
	FailedEvents       int     `json:"failed_events"`
	ProgressPercentage float64 `json:"progress_percentage"`
	EventsPerSecond    float64 `json:"events_per_second"`
	ErrorMessage       *string `json:"error_message"`
}

// APIError represents an error returned by the AllSource API.
type APIError struct {
	Code       string `json:"code"`
	Message    string `json:"message"`
	StatusCode int    `json:"status_code"`
}

func (e *APIError) Error() string {
	return fmt.Sprintf("allsource: %s (status %d): %s", e.Code, e.StatusCode, e.Message)
}

// IsUnauthorized returns true if the error is a 401 Unauthorized.
func (e *APIError) IsUnauthorized() bool { return e.StatusCode == 401 }

// IsForbidden returns true if the error is a 403 Forbidden.
func (e *APIError) IsForbidden() bool { return e.StatusCode == 403 }

// IsNotFound returns true if the error is a 404 Not Found.
func (e *APIError) IsNotFound() bool { return e.StatusCode == 404 }

// IsRateLimited returns true if the error is a 429 Too Many Requests.
func (e *APIError) IsRateLimited() bool { return e.StatusCode == 429 }

// IsServerError returns true if the error is a 5xx server error.
func (e *APIError) IsServerError() bool { return e.StatusCode >= 500 }

// IsRetryable returns true if the error has a status code that is safe to retry.
func (e *APIError) IsRetryable() bool {
	switch e.StatusCode {
	case 408, 429, 500, 502, 503, 504:
		return true
	default:
		return false
	}
}
