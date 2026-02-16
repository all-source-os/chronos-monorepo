package allsource

import "fmt"

// Event represents an event in the AllSource event store.
type Event struct {
	ID        string         `json:"id"`
	EntityID  string         `json:"entity_id"`
	EventType string         `json:"event_type"`
	Payload   map[string]any `json:"payload"`
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
