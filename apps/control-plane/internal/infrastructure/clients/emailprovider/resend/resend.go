// Package resend implements the email connector against the Resend API
// (https://resend.com). Unlike Nylas (OAuth grants into existing third-party
// mailboxes), Resend manages email for domains you control: send from a verified
// domain, receive via MX→Resend with a thin `email.received` webhook (email_id
// only) that we resolve by fetching the message by id. The connector is
// provider-neutral downstream — it emits the same normalized email.* shapes as
// Nylas, so Core events / Prime / the admin UI are unchanged.
//
// Model mapping vs Nylas:
//   - "grant" → the receiving address itself (e.g. sales@all-source.xyz). There
//     is no OAuth grant; a connection is just a verified address. Send uses it as
//     the From; the webhook resolves the tenant by the To address.
//   - message entity_id → the RFC Message-ID, so a reply's In-Reply-To header
//     (set from the inbound entity_id) threads correctly.
package resend

import (
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"fmt"
	"net/http"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/go-resty/resty/v2"

	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider"
)

const (
	defaultBaseURL    = "https://api.resend.com"
	webhookTolerance  = 5 * time.Minute // Svix replay window
	snippetMaxChars   = 240
	defaultHTTPTimeut = 30 * time.Second
)

// Provider is a Resend implementation of the email connector. It is NOT bound to
// emailprovider.Provider (Resend has no OAuth/grant surface); it implements the
// inboxSender slice (Name/Send/ListGrants) the admin handler needs, plus
// FetchMessage + VerifyWebhook for the inbound webhook handler.
type Provider struct {
	client        *resty.Client
	apiKey        string
	webhookSecret string // Svix signing secret (whsec_…)
}

// New builds a Provider from explicit config.
func New(apiKey, webhookSecret string) *Provider {
	client := resty.New().
		SetBaseURL(defaultBaseURL).
		SetAuthToken(apiKey).
		SetHeader("Content-Type", "application/json").
		SetTimeout(defaultHTTPTimeut)
	return &Provider{client: client, apiKey: apiKey, webhookSecret: webhookSecret}
}

// NewFromEnv builds a Provider from RESEND_API_KEY and RESEND_WEBHOOK_SECRET.
// Returns nil when RESEND_API_KEY is unset (connector disabled).
func NewFromEnv() *Provider {
	key := os.Getenv("RESEND_API_KEY")
	if key == "" {
		return nil
	}
	return New(key, os.Getenv("RESEND_WEBHOOK_SECRET"))
}

// HasWebhookSecret reports whether inbound signature verification is configured.
func (p *Provider) HasWebhookSecret() bool { return p.webhookSecret != "" }

// Name implements the connector contract.
func (p *Provider) Name() string { return "resend" }

// --- Send ---

// Send sends through the verified domain. grantID is the connected From address.
// In-Reply-To / References headers thread the reply (set from the inbound
// message's RFC Message-ID, which we store as the event entity_id).
func (p *Provider) Send(ctx context.Context, grantID string, req emailprovider.SendRequest) (*emailprovider.SendResult, error) {
	body := map[string]any{
		"from":    grantID,
		"to":      emails(req.To),
		"subject": req.Subject,
	}
	if strings.TrimSpace(req.Body) != "" {
		body["text"] = req.Body
	}
	if len(req.Cc) > 0 {
		body["cc"] = emails(req.Cc)
	}
	if len(req.Bcc) > 0 {
		body["bcc"] = emails(req.Bcc)
	}
	if req.InReplyTo != "" {
		body["headers"] = map[string]string{
			"In-Reply-To": req.InReplyTo,
			"References":  req.InReplyTo,
		}
	}
	var out struct {
		ID string `json:"id"`
	}
	resp, err := p.client.R().
		SetContext(ctx).
		SetBody(body).
		SetResult(&out).
		Post("/emails")
	if err != nil {
		return nil, fmt.Errorf("resend: send: %w", err)
	}
	if resp.IsError() {
		return nil, fmt.Errorf("resend: send: status %d: %s", resp.StatusCode(), resp.String())
	}
	return &emailprovider.SendResult{
		MessageID: out.ID,
		ThreadID:  req.ThreadID, // Resend has no thread id; echo the caller's
		SentAt:    time.Now().UTC(),
	}, nil
}

// ListGrants satisfies the inboxSender slice. Resend has no enumerable grants
// (a connection is an address we register), so this is always empty.
func (p *Provider) ListGrants(_ context.Context) ([]emailprovider.Grant, error) {
	return []emailprovider.Grant{}, nil
}

// --- Inbound: fetch full content (the webhook is thin: email_id only) ---

type receivedMessage struct {
	ID        string            `json:"id"`
	MessageID string            `json:"message_id"` // RFC Message-ID
	From      string            `json:"from"`
	To        []string          `json:"to"`
	Cc        []string          `json:"cc"`
	Subject   string            `json:"subject"`
	HTML      string            `json:"html"`
	Text      string            `json:"text"`
	Headers   map[string]string `json:"headers"`
	CreatedAt string            `json:"created_at"`
}

// FetchMessage retrieves a received email's full content by Resend email id and
// normalizes it. The returned Message.ID is the RFC Message-ID (stable, and what
// a reply's In-Reply-To references).
func (p *Provider) FetchMessage(ctx context.Context, _grantID, emailID string) (*emailprovider.Message, error) {
	var out receivedMessage
	resp, err := p.client.R().
		SetContext(ctx).
		SetResult(&out).
		Get("/emails/receiving/" + emailID)
	if err != nil {
		return nil, fmt.Errorf("resend: fetch received: %w", err)
	}
	if resp.IsError() {
		return nil, fmt.Errorf("resend: fetch received %s: status %d: %s", emailID, resp.StatusCode(), resp.String())
	}
	msg := normalize(out)
	return &msg, nil
}

func normalize(r receivedMessage) emailprovider.Message {
	id := strings.TrimSpace(r.MessageID)
	if id == "" {
		id = r.ID
	}
	body := r.Text
	if strings.TrimSpace(body) == "" {
		body = r.HTML
	}
	return emailprovider.Message{
		ID:         id,
		ThreadID:   deriveThreadID(r.Headers, id),
		Subject:    r.Subject,
		From:       parseAddress(r.From),
		To:         parseAddresses(r.To),
		Cc:         parseAddresses(r.Cc),
		Snippet:    snippet(body),
		Body:       body,
		ReceivedAt: parseTime(r.CreatedAt),
	}
}

// deriveThreadID reconstructs a stable thread id from RFC headers (Resend has no
// native thread id): the first id in References is the thread root; else the
// In-Reply-To target; else the message's own id (a new thread).
func deriveThreadID(headers map[string]string, selfID string) string {
	if refs := headerGet(headers, "References"); refs != "" {
		if first := strings.Fields(refs); len(first) > 0 {
			return first[0]
		}
	}
	if irt := strings.TrimSpace(headerGet(headers, "In-Reply-To")); irt != "" {
		return irt
	}
	return selfID
}

func headerGet(headers map[string]string, key string) string {
	if headers == nil {
		return ""
	}
	for k, v := range headers {
		if strings.EqualFold(k, key) {
			return v
		}
	}
	return ""
}

// --- Webhook signature (Svix) ---

// VerifyWebhook verifies a Svix-signed Resend webhook: HMAC-SHA256 over
// "<svix-id>.<svix-timestamp>.<body>" with the base64-decoded whsec_ key, matched
// (constant-time) against any v1 signature in the svix-signature header, within a
// replay tolerance. Fails closed when the secret is unset.
func (p *Provider) VerifyWebhook(headers http.Header, body []byte) bool {
	if p.webhookSecret == "" {
		return false
	}
	id := headers.Get("svix-id")
	ts := headers.Get("svix-timestamp")
	sigHeader := headers.Get("svix-signature")
	if id == "" || ts == "" || sigHeader == "" {
		return false
	}
	// Replay window: reject timestamps outside ±tolerance.
	if secs, err := strconv.ParseInt(ts, 10, 64); err == nil {
		delta := time.Since(time.Unix(secs, 0))
		if delta < 0 {
			delta = -delta
		}
		if delta > webhookTolerance {
			return false
		}
	} else {
		return false
	}
	key, err := base64.StdEncoding.DecodeString(strings.TrimPrefix(p.webhookSecret, "whsec_"))
	if err != nil {
		return false
	}
	mac := hmac.New(sha256.New, key)
	mac.Write([]byte(id + "." + ts + "." + string(body)))
	expected := base64.StdEncoding.EncodeToString(mac.Sum(nil))
	for _, part := range strings.Fields(sigHeader) {
		sig := part
		if i := strings.IndexByte(part, ','); i >= 0 {
			sig = part[i+1:] // strip the "v1," version prefix
		}
		if hmac.Equal([]byte(sig), []byte(expected)) {
			return true
		}
	}
	return false
}

// --- helpers ---

func emails(addrs []emailprovider.Address) []string {
	out := make([]string, 0, len(addrs))
	for _, a := range addrs {
		if a.Email != "" {
			out = append(out, a.Email)
		}
	}
	return out
}

// parseAddress parses "Name <email@x.com>" or a bare "email@x.com".
func parseAddress(s string) emailprovider.Address {
	s = strings.TrimSpace(s)
	if i := strings.LastIndex(s, "<"); i >= 0 {
		if j := strings.IndexByte(s[i:], '>'); j >= 0 {
			name := strings.TrimSpace(strings.Trim(s[:i], `" `))
			return emailprovider.Address{Name: name, Email: strings.TrimSpace(s[i+1 : i+j])}
		}
	}
	return emailprovider.Address{Email: s}
}

func parseAddresses(in []string) []emailprovider.Address {
	out := make([]emailprovider.Address, 0, len(in))
	for _, s := range in {
		if a := parseAddress(s); a.Email != "" {
			out = append(out, a)
		}
	}
	return out
}

func snippet(body string) string {
	s := strings.TrimSpace(body)
	if len(s) > snippetMaxChars {
		return strings.TrimSpace(s[:snippetMaxChars]) + "…"
	}
	return s
}

func parseTime(s string) time.Time {
	if t, err := time.Parse(time.RFC3339, s); err == nil {
		return t.UTC()
	}
	return time.Now().UTC()
}
