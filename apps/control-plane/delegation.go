package main

// Data-plane delegation. Control Plane is the single public entry point for
// chronis and the SDKs. After AuthMiddleware validates the caller (JWT or
// ask_ API key), these handlers forward the request to the correct internal
// backend — writes to Core, reads to Query Service — with the authenticated
// tenant_id injected from the caller's identity. The forwarded request is
// authenticated with Control Plane's service JWT, so backends never see
// public auth material and never need to validate ask_ keys themselves.

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"

	"github.com/gin-gonic/gin"
)

// delegationClient holds the resolved backend URLs and the shared HTTP
// client used to forward requests. serviceToken is Control Plane's long-lived
// admin JWT — backends trust it and authorize the request on CP's behalf.
type delegationClient struct {
	core         *url.URL
	queryService *url.URL
	serviceToken string
	http         *http.Client
}

func newDelegationClient(coreURL, queryURL, serviceToken string, httpClient *http.Client) (*delegationClient, error) {
	if serviceToken == "" {
		return nil, errors.New("delegation requires serviceToken")
	}
	core, err := url.Parse(strings.TrimRight(coreURL, "/"))
	if err != nil {
		return nil, fmt.Errorf("parse core URL: %w", err)
	}
	qs, err := url.Parse(strings.TrimRight(queryURL, "/"))
	if err != nil {
		return nil, fmt.Errorf("parse query-service URL: %w", err)
	}
	if httpClient == nil {
		httpClient = http.DefaultClient
	}
	return &delegationClient{
		core:         core,
		queryService: qs,
		serviceToken: serviceToken,
		http:         httpClient,
	}, nil
}

// authTenantFromContext returns the authenticated caller's tenant_id set by
// AuthMiddleware. Delegation requires a tenant — unauthenticated or
// tenantless callers cannot reach these routes. Returns false when absent
// so handlers can 401 rather than forwarding an un-scoped write.
func authTenantFromContext(c *gin.Context) (string, bool) {
	v, ok := c.Get("auth_tenant_id")
	if !ok {
		return "", false
	}
	s, ok := v.(string)
	if !ok || s == "" {
		return "", false
	}
	return s, true
}

// injectTenantIntoObject overwrites the top-level tenant_id field in a JSON
// object. Used for single-event ingest where the Core handler reads
// req.tenant_id.
func injectTenantIntoObject(body []byte, tenantID string) ([]byte, error) {
	var doc map[string]any
	if len(body) == 0 {
		doc = map[string]any{}
	} else if err := json.Unmarshal(body, &doc); err != nil {
		return nil, fmt.Errorf("decode ingest body: %w", err)
	}
	doc["tenant_id"] = tenantID
	return json.Marshal(doc)
}

// injectTenantIntoBatch overwrites tenant_id on every element in a batch
// request's "events" array. Callers that submit a pre-populated tenant per
// event still get overwritten — trusting the gateway is the whole point.
func injectTenantIntoBatch(body []byte, tenantID string) ([]byte, error) {
	var doc map[string]any
	if err := json.Unmarshal(body, &doc); err != nil {
		return nil, fmt.Errorf("decode batch body: %w", err)
	}
	events, ok := doc["events"].([]any)
	if !ok {
		return nil, errors.New("batch body missing 'events' array")
	}
	for i, raw := range events {
		ev, ok := raw.(map[string]any)
		if !ok {
			return nil, fmt.Errorf("events[%d] is not a JSON object", i)
		}
		ev["tenant_id"] = tenantID
		events[i] = ev
	}
	doc["events"] = events
	return json.Marshal(doc)
}

// forwardRequest builds a new outgoing request against backend, copies the
// body, attaches the service JWT, and copies the backend's response to the
// client. It does NOT forward any inbound auth headers — the backend trusts
// Control Plane, not the caller.
func (d *delegationClient) forwardRequest(c *gin.Context, method string, backend *url.URL, path string, query url.Values, body []byte) {
	target := *backend
	target.Path = strings.TrimRight(backend.Path, "/") + "/" + strings.TrimLeft(path, "/")
	if query != nil {
		target.RawQuery = query.Encode()
	}

	var bodyReader io.Reader
	if body != nil {
		bodyReader = bytes.NewReader(body)
	}

	req, err := http.NewRequestWithContext(c.Request.Context(), method, target.String(), bodyReader)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "delegation build request", "message": err.Error()})
		return
	}
	req.Header.Set("Authorization", "Bearer "+d.serviceToken)
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}

	resp, err := d.http.Do(req)
	if err != nil {
		c.JSON(http.StatusBadGateway, gin.H{"error": "delegation upstream", "message": err.Error()})
		return
	}
	defer resp.Body.Close()

	// Copy status + response body. Content-Type comes from the backend so
	// JSON responses carry through unchanged.
	if ct := resp.Header.Get("Content-Type"); ct != "" {
		c.Writer.Header().Set("Content-Type", ct)
	}
	c.Writer.WriteHeader(resp.StatusCode)
	if _, err := io.Copy(c.Writer, resp.Body); err != nil {
		// Response headers already sent — best we can do is log.
		_ = err
	}
}

// ProxyIngestSingle forwards POST /api/v1/events to Core with tenant_id
// injected into the JSON body from the authenticated caller.
func (cp *ControlPlane) ProxyIngestSingle(c *gin.Context) {
	tenantID, ok := authTenantFromContext(c)
	if !ok {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "unauthorized", "message": "no tenant context"})
		return
	}
	body, err := io.ReadAll(c.Request.Body)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "bad request", "message": err.Error()})
		return
	}
	rewritten, err := injectTenantIntoObject(body, tenantID)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "bad request", "message": err.Error()})
		return
	}
	cp.delegation.forwardRequest(c, http.MethodPost, cp.delegation.core, "/api/v1/events", nil, rewritten)
}

// ProxyIngestBatch forwards POST /api/v1/events/batch to Core with tenant_id
// injected onto every event in the batch.
func (cp *ControlPlane) ProxyIngestBatch(c *gin.Context) {
	tenantID, ok := authTenantFromContext(c)
	if !ok {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "unauthorized", "message": "no tenant context"})
		return
	}
	body, err := io.ReadAll(c.Request.Body)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "bad request", "message": err.Error()})
		return
	}
	rewritten, err := injectTenantIntoBatch(body, tenantID)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "bad request", "message": err.Error()})
		return
	}
	cp.delegation.forwardRequest(c, http.MethodPost, cp.delegation.core, "/api/v1/events/batch", nil, rewritten)
}

// ProxyEventsQuery forwards GET /api/v1/events/query to Query Service with
// tenant_id injected as a query param. Any client-supplied tenant_id is
// overwritten — the authenticated caller's tenant is authoritative.
func (cp *ControlPlane) ProxyEventsQuery(c *gin.Context) {
	tenantID, ok := authTenantFromContext(c)
	if !ok {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "unauthorized", "message": "no tenant context"})
		return
	}
	q := c.Request.URL.Query()
	q.Set("tenant_id", tenantID)
	cp.delegation.forwardRequest(c, http.MethodGet, cp.delegation.queryService, "/api/v1/events/query", q, nil)
}
