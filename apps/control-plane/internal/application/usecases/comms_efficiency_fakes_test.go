package usecases

import (
	"context"
	"sort"
	"sync"
	"time"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// effCore is a capable in-memory fake CoreClient for the comms-efficiency tests.
// It stores events (comms instrumentation under admin-comms; goal events under
// each customer tenant) and answers QueryEvents with tenant/type/time/order/limit
// filtering — enough to exercise the real join + attribution. It also implements
// first-ingest (ExpectedVersion) dedupe so the engagement idempotency path is real.
type effCore struct {
	clients.CoreClient // embedded: unused methods panic if called (none are)

	mu             sync.Mutex
	events         []effEvent
	config         map[string]string
	entityVersions map[string]uint64 // "tenant|entityID" -> current version
	churned        map[string]bool   // tenant -> QueryEvents returns an error (deleted)
	ingests        int
}

type effEvent struct {
	tenant    string
	eventType string
	entityID  string
	ts        time.Time
	payload   map[string]any
}

func newEffCore() *effCore {
	return &effCore{config: map[string]string{}, entityVersions: map[string]uint64{}, churned: map[string]bool{}}
}

// addEvent inserts an event with an explicit timestamp (full control over the
// timeline the attribution windows are tested against).
func (c *effCore) addEvent(tenant, eventType, entityID string, ts time.Time, payload map[string]any) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.events = append(c.events, effEvent{tenant: tenant, eventType: eventType, entityID: entityID, ts: ts.UTC(), payload: payload})
}

// markChurned makes QueryEvents for a tenant return an error (deleted/unreadable).
func (c *effCore) markChurned(tenant string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.churned[tenant] = true
}

func (c *effCore) QueryEvents(_ context.Context, req clients.QueryEventsRequest) (*clients.QueryEventsResponse, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.churned[req.TenantID] {
		return nil, context.DeadlineExceeded // any error → reconciler treats as churned/unreadable
	}
	var since, until time.Time
	if req.Since != "" {
		since, _ = time.Parse(time.RFC3339, req.Since)
	}
	if req.Until != "" {
		until, _ = time.Parse(time.RFC3339, req.Until)
	}
	var matched []effEvent
	for _, e := range c.events {
		if e.tenant != req.TenantID {
			continue
		}
		if req.EventType != "" && e.eventType != req.EventType {
			continue
		}
		if !since.IsZero() && e.ts.Before(since) {
			continue
		}
		if !until.IsZero() && e.ts.After(until) {
			continue
		}
		matched = append(matched, e)
	}
	sort.Slice(matched, func(i, j int) bool { return matched[i].ts.Before(matched[j].ts) })
	if req.Order == "desc" {
		for i, j := 0, len(matched)-1; i < j; i, j = i+1, j-1 {
			matched[i], matched[j] = matched[j], matched[i]
		}
	}
	total := len(matched)
	if req.Offset > 0 {
		if req.Offset >= len(matched) {
			matched = nil
		} else {
			matched = matched[req.Offset:]
		}
	}
	if req.Limit > 0 && len(matched) > req.Limit {
		matched = matched[:req.Limit]
	}
	out := make([]clients.EventEntry, 0, len(matched))
	for _, e := range matched {
		out = append(out, clients.EventEntry{
			ID: e.entityID, EventType: e.eventType, EntityID: e.entityID,
			Timestamp: e.ts.Format(time.RFC3339Nano), Payload: e.payload,
		})
	}
	return &clients.QueryEventsResponse{Events: out, Count: len(out), TotalCount: total}, nil
}

func (c *effCore) IngestEvent(_ context.Context, req clients.IngestEventRequest) (*clients.IngestEventResponse, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.ingests++
	key := req.TenantID + "|" + req.EntityID
	if req.ExpectedVersion != nil {
		if c.entityVersions[key] != *req.ExpectedVersion {
			return nil, clients.ErrVersionConflict // replayed first-ingest → idempotent drop
		}
	}
	c.entityVersions[key]++
	ts := time.Now().UTC()
	c.events = append(c.events, effEvent{tenant: req.TenantID, eventType: req.EventType, entityID: req.EntityID, ts: ts, payload: req.Payload})
	return &clients.IngestEventResponse{ID: "ev"}, nil
}

func (c *effCore) SetConfig(_ context.Context, req clients.SetConfigRequest) error {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.config == nil {
		c.config = map[string]string{}
	}
	if s, ok := req.Value.(string); ok {
		c.config[req.Key] = s
	}
	return nil
}

func (c *effCore) GetConfig(_ context.Context, key string) (*clients.ConfigEntryResponse, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	v, ok := c.config[key]
	if !ok {
		return nil, nil
	}
	return &clients.ConfigEntryResponse{Key: key, Value: v}, nil
}

// sendEvent stamps a send (admin.message.sent) under admin-comms with the given
// tags + send_ts — the shape the reconciler reads. skipped=false counts as a send.
func (c *effCore) sendEvent(tags CommsTags) {
	p := map[string]any{"skipped": false, "template": "t"}
	tags.ApplyTo(p)
	c.addEvent(CommsAuditTenant, MessageSentEventType, "message:"+tags.TenantID, mustTime(tags.SendTS), p)
}

func (c *effCore) holdoutEvent(tags CommsTags) {
	tags.Holdout = true
	p := map[string]any{"skipped": true, "skip_reason": SkipHeldOut}
	tags.ApplyTo(p)
	c.addEvent(CommsAuditTenant, HoldoutEventType, "holdout:"+tags.TenantID, mustTime(tags.SendTS), p)
}

func (c *effCore) engageEvent(eventType string, tags CommsTags) {
	p := map[string]any{}
	tags.ApplyTo(p)
	c.addEvent(CommsAuditTenant, eventType, commsEngagementEntityID(tags.MessageID, eventType), mustTime(tags.SendTS), p)
}

func mustTime(rfc string) time.Time {
	t, _ := time.Parse(time.RFC3339, rfc)
	return t.UTC()
}

func rfc(t time.Time) string { return t.UTC().Format(time.RFC3339) }
