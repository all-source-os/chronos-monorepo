package allsource

import "context"

// EventFolder reconstructs domain state from an event stream.
// S is the type of the domain state being built.
type EventFolder[S any] interface {
	// Apply processes a single event. Returns true if the event was relevant.
	Apply(event Event) bool
	// Finalize returns the accumulated domain state and whether any events were relevant.
	Finalize() (S, bool)
}

// FoldEvents folds a slice of events using an EventFolder.
func FoldEvents[S any](folder EventFolder[S], events []Event) (S, bool) {
	for _, e := range events {
		folder.Apply(e)
	}
	return folder.Finalize()
}

// QueryAndFold queries events from the AllSource API and folds them into domain state.
// This is a package-level function because Go methods cannot have type parameters.
func QueryAndFold[S any](ctx context.Context, c *Client, opts QueryOptions, folder EventFolder[S]) (S, bool, error) {
	result, err := c.Query(ctx, opts)
	if err != nil {
		var zero S
		return zero, false, err
	}
	state, ok := FoldEvents(folder, result.Events)
	return state, ok, nil
}
