package http

import (
	"encoding/json"
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/application/dto"
)

func TestTenantSelfLinks(t *testing.T) {
	links := tenantSelfLinks("t-123")

	if len(links) != 1 {
		t.Fatalf("expected 1 link, got %d", len(links))
	}
	if links["self"].Href != "/api/v1/tenants/t-123" {
		t.Errorf("unexpected self href: %s", links["self"].Href)
	}
}

func TestTenantDetailLinks(t *testing.T) {
	t.Setenv("DATA_PLANE_URL", "http://query:3902")

	links := tenantDetailLinks("t-456")

	expectedLinks := map[string]string{
		"self":    "/api/v1/tenants/t-456",
		"stats":   "/api/v1/tenants/t-456/stats",
		"usage":   "/api/v1/tenants/t-456/usage",
		"billing": "/api/v1/tenants/t-456/billing",
		"audit":   "/api/v1/tenants/t-456/audit",
		"events":  "http://query:3902/api/v1/events{?stream_id,event_type}",
		"schemas": "http://query:3902/api/v1/schemas{?name}",
	}

	if len(links) != len(expectedLinks) {
		t.Fatalf("expected %d links, got %d", len(expectedLinks), len(links))
	}

	for name, expectedHref := range expectedLinks {
		link, ok := links[name]
		if !ok {
			t.Errorf("missing link: %s", name)
			continue
		}
		if link.Href != expectedHref {
			t.Errorf("link %s: expected href %q, got %q", name, expectedHref, link.Href)
		}
	}

	// events and schemas should be templated
	if !links["events"].Templated {
		t.Error("expected events link to be templated")
	}
	if !links["schemas"].Templated {
		t.Error("expected schemas link to be templated")
	}

	// self should not be templated
	if links["self"].Templated {
		t.Error("expected self link to not be templated")
	}
}

func TestTenantDetailLinksNoDataPlaneURL(t *testing.T) {
	t.Setenv("DATA_PLANE_URL", "")

	links := tenantDetailLinks("t-789")

	// events/schemas should still work with empty base
	if links["events"].Href != "/api/v1/events{?stream_id,event_type}" {
		t.Errorf("unexpected events href without DATA_PLANE_URL: %s", links["events"].Href)
	}
	if links["schemas"].Href != "/api/v1/schemas{?name}" {
		t.Errorf("unexpected schemas href without DATA_PLANE_URL: %s", links["schemas"].Href)
	}
}

func TestWrapTenantWithSelfLinkJSON(t *testing.T) {
	resp := &dto.TenantResponse{
		ID:        "t-001",
		Name:      "Test",
		Status:    "active",
		CreatedAt: time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC),
		UpdatedAt: time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC),
	}

	wrapped := wrapTenantWithSelfLink(resp)
	data, err := json.Marshal(wrapped)
	if err != nil {
		t.Fatalf("marshal error: %v", err)
	}

	var parsed map[string]interface{}
	if err := json.Unmarshal(data, &parsed); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	// Verify _links.self exists
	links, ok := parsed["_links"].(map[string]interface{})
	if !ok {
		t.Fatalf("_links not found in JSON: %s", string(data))
	}
	selfLink, ok := links["self"].(map[string]interface{})
	if !ok {
		t.Fatal("self link not found")
	}
	if selfLink["href"] != "/api/v1/tenants/t-001" {
		t.Errorf("unexpected self href: %v", selfLink["href"])
	}

	// Verify existing fields are preserved
	if parsed["id"] != "t-001" {
		t.Errorf("expected id t-001, got %v", parsed["id"])
	}
	if parsed["name"] != "Test" {
		t.Errorf("expected name Test, got %v", parsed["name"])
	}
	if parsed["status"] != "active" {
		t.Errorf("expected status active, got %v", parsed["status"])
	}
}

func TestWrapTenantWithDetailLinksJSON(t *testing.T) {
	t.Setenv("DATA_PLANE_URL", "http://dp:3902")

	resp := &dto.TenantResponse{
		ID:     "t-002",
		Name:   "Detail",
		Status: "active",
	}

	wrapped := wrapTenantWithDetailLinks(resp)
	data, err := json.Marshal(wrapped)
	if err != nil {
		t.Fatalf("marshal error: %v", err)
	}

	var parsed map[string]interface{}
	if err := json.Unmarshal(data, &parsed); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	links, ok := parsed["_links"].(map[string]interface{})
	if !ok {
		t.Fatalf("_links not found in JSON: %s", string(data))
	}

	// All 7 links should be present
	expectedNames := []string{"self", "stats", "usage", "billing", "audit", "events", "schemas"}
	for _, name := range expectedNames {
		if _, ok := links[name]; !ok {
			t.Errorf("missing link: %s", name)
		}
	}

	// Verify templated links have templated=true
	eventsLink := links["events"].(map[string]interface{})
	if eventsLink["templated"] != true {
		t.Error("expected events link to be templated")
	}
	schemasLink := links["schemas"].(map[string]interface{})
	if schemasLink["templated"] != true {
		t.Error("expected schemas link to be templated")
	}

	// Existing fields still present
	if parsed["id"] != "t-002" {
		t.Errorf("expected id t-002, got %v", parsed["id"])
	}
}
