package http

import (
	"encoding/json"
	"testing"
)

func TestSelfLink(t *testing.T) {
	link := SelfLink("/api/v1/tenants/123")
	if link.Href != "/api/v1/tenants/123" {
		t.Errorf("expected href /api/v1/tenants/123, got %s", link.Href)
	}
	if link.Title != "" {
		t.Errorf("expected empty title, got %s", link.Title)
	}
	if link.Templated {
		t.Error("expected templated to be false")
	}
}

func TestNewLink(t *testing.T) {
	link := NewLink("/api/v1/events", WithTitle("Events"), WithTemplated())
	if link.Href != "/api/v1/events" {
		t.Errorf("expected href /api/v1/events, got %s", link.Href)
	}
	if link.Title != "Events" {
		t.Errorf("expected title Events, got %s", link.Title)
	}
	if !link.Templated {
		t.Error("expected templated to be true")
	}
}

func TestNewLinkNoOptions(t *testing.T) {
	link := NewLink("/path")
	if link.Href != "/path" {
		t.Errorf("expected href /path, got %s", link.Href)
	}
	if link.Title != "" || link.Templated {
		t.Error("expected no title and not templated")
	}
}

func TestMergeLinks(t *testing.T) {
	a := map[string]Link{
		"self":    SelfLink("/a"),
		"tenants": NewLink("/tenants"),
	}
	b := map[string]Link{
		"self":   SelfLink("/b"),
		"events": NewLink("/events"),
	}

	merged := MergeLinks(a, b)

	if len(merged) != 3 {
		t.Errorf("expected 3 links, got %d", len(merged))
	}
	if merged["self"].Href != "/b" {
		t.Errorf("expected self to be /b (later override), got %s", merged["self"].Href)
	}
	if merged["tenants"].Href != "/tenants" {
		t.Errorf("expected tenants link preserved, got %s", merged["tenants"].Href)
	}
	if merged["events"].Href != "/events" {
		t.Errorf("expected events link from b, got %s", merged["events"].Href)
	}
}

func TestMergeLinksEmpty(t *testing.T) {
	merged := MergeLinks()
	if len(merged) != 0 {
		t.Errorf("expected empty map, got %d entries", len(merged))
	}
}

func TestDataPlaneLink(t *testing.T) {
	t.Setenv("DATA_PLANE_URL", "http://query-service:3902")

	link := DataPlaneLink("/api/v1/events{?stream_id}", WithTitle("Query Events"))
	if link.Href != "http://query-service:3902/api/v1/events{?stream_id}" {
		t.Errorf("unexpected href: %s", link.Href)
	}
	if !link.Templated {
		t.Error("expected templated to be true")
	}
	if link.Title != "Query Events" {
		t.Errorf("expected title Query Events, got %s", link.Title)
	}
}

func TestDataPlaneLinkNoEnv(t *testing.T) {
	t.Setenv("DATA_PLANE_URL", "")

	link := DataPlaneLink("/api/v1/events")
	if link.Href != "/api/v1/events" {
		t.Errorf("expected href with empty base, got %s", link.Href)
	}
}

func TestHALResourceJSON(t *testing.T) {
	type TenantResponse struct {
		HALResource
		ID   string `json:"id"`
		Name string `json:"name"`
	}

	resp := TenantResponse{
		HALResource: HALResource{
			Links: map[string]Link{
				"self":   SelfLink("/api/v1/tenants/t1"),
				"events": NewLink("/api/v1/events{?tenant_id}", WithTemplated(), WithTitle("Tenant Events")),
			},
		},
		ID:   "t1",
		Name: "Test Tenant",
	}

	data, err := json.Marshal(resp)
	if err != nil {
		t.Fatalf("marshal error: %v", err)
	}

	var parsed map[string]interface{}
	if err := json.Unmarshal(data, &parsed); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	links, ok := parsed["_links"].(map[string]interface{})
	if !ok {
		t.Fatalf("_links not found or wrong type in JSON: %s", string(data))
	}

	selfLink, ok := links["self"].(map[string]interface{})
	if !ok {
		t.Fatal("self link not found")
	}
	if selfLink["href"] != "/api/v1/tenants/t1" {
		t.Errorf("unexpected self href: %v", selfLink["href"])
	}

	eventsLink, ok := links["events"].(map[string]interface{})
	if !ok {
		t.Fatal("events link not found")
	}
	if eventsLink["templated"] != true {
		t.Error("expected events link to be templated")
	}
	if eventsLink["title"] != "Tenant Events" {
		t.Errorf("unexpected events title: %v", eventsLink["title"])
	}

	if parsed["id"] != "t1" {
		t.Errorf("expected id t1, got %v", parsed["id"])
	}
}

func TestHALResourceOmitsEmptyLinks(t *testing.T) {
	type Response struct {
		HALResource
		Name string `json:"name"`
	}

	resp := Response{Name: "test"}
	data, err := json.Marshal(resp)
	if err != nil {
		t.Fatalf("marshal error: %v", err)
	}

	var parsed map[string]interface{}
	if err := json.Unmarshal(data, &parsed); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if _, exists := parsed["_links"]; exists {
		t.Error("expected _links to be omitted when nil")
	}
}

func TestLinkJSONOmitsZeroFields(t *testing.T) {
	link := SelfLink("/test")
	data, err := json.Marshal(link)
	if err != nil {
		t.Fatalf("marshal error: %v", err)
	}

	var parsed map[string]interface{}
	if err := json.Unmarshal(data, &parsed); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if _, exists := parsed["title"]; exists {
		t.Error("expected title to be omitted")
	}
	if _, exists := parsed["templated"]; exists {
		t.Error("expected templated to be omitted")
	}
}
