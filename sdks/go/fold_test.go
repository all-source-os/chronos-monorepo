package allsource

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

// orderTotalFolder sums up order amounts from events.
type orderTotalFolder struct {
	total   float64
	counted int
}

func (f *orderTotalFolder) Apply(e Event) bool {
	if e.EventType != "order.placed" {
		return false
	}
	if amt, ok := e.Payload["amount"].(float64); ok {
		f.total += amt
		f.counted++
		return true
	}
	return false
}

func (f *orderTotalFolder) Finalize() (float64, bool) {
	if f.counted == 0 {
		return 0, false
	}
	return f.total, true
}

func TestFoldEventsBasic(t *testing.T) {
	events := []Event{
		{EventType: "order.placed", Payload: map[string]any{"amount": float64(10)}},
		{EventType: "order.placed", Payload: map[string]any{"amount": float64(25)}},
		{EventType: "order.shipped", Payload: map[string]any{"tracking": "ABC"}},
		{EventType: "order.placed", Payload: map[string]any{"amount": float64(5)}},
	}

	folder := &orderTotalFolder{}
	total, ok := FoldEvents[float64](folder, events)
	if !ok {
		t.Fatal("expected ok=true")
	}
	if total != 40 {
		t.Errorf("expected total 40, got %f", total)
	}
}

func TestFoldEventsEmpty(t *testing.T) {
	folder := &orderTotalFolder{}
	total, ok := FoldEvents[float64](folder, nil)
	if ok {
		t.Error("expected ok=false for empty events")
	}
	if total != 0 {
		t.Errorf("expected total 0, got %f", total)
	}
}

func TestFoldEventsNoRelevant(t *testing.T) {
	events := []Event{
		{EventType: "user.signup", Payload: map[string]any{"plan": "free"}},
		{EventType: "user.login", Payload: map[string]any{}},
	}

	folder := &orderTotalFolder{}
	total, ok := FoldEvents[float64](folder, events)
	if ok {
		t.Error("expected ok=false when no relevant events")
	}
	if total != 0 {
		t.Errorf("expected total 0, got %f", total)
	}
}

func TestQueryAndFold(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]any{
			"events": []map[string]any{
				{"event_type": "order.placed", "payload": map[string]any{"amount": float64(15)}},
				{"event_type": "order.placed", "payload": map[string]any{"amount": float64(30)}},
			},
			"count": 2,
		})
	}))
	defer srv.Close()

	client := New("key", srv.URL)
	folder := &orderTotalFolder{}

	total, ok, err := QueryAndFold[float64](context.Background(), client, QueryOptions{}, folder)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !ok {
		t.Fatal("expected ok=true")
	}
	if total != 45 {
		t.Errorf("expected total 45, got %f", total)
	}
}

func TestQueryAndFoldError(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(500)
		json.NewEncoder(w).Encode(map[string]any{
			"error": map[string]any{"code": "internal", "message": "fail"},
		})
	}))
	defer srv.Close()

	client := New("key", srv.URL)
	folder := &orderTotalFolder{}

	_, _, err := QueryAndFold[float64](context.Background(), client, QueryOptions{}, folder)
	if err == nil {
		t.Fatal("expected error")
	}
}
