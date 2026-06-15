// Package nylas implements emailprovider.Provider against the Nylas Email API v3
// (the grant-per-mailbox model). Field names track the Nylas v3 REST docs;
// verify against the current docs before production. See
// docs/proposals/AI_INBOX_ON_ALLSOURCE.md §3.1.
package nylas

import (
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"net/url"
	"os"
	"strings"
	"time"

	"github.com/go-resty/resty/v2"

	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider"
)

const defaultBaseURL = "https://api.us.nylas.com"

// Config configures a Nylas Provider.
type Config struct {
	APIKey        string // Nylas application API key (Bearer + hosted-auth client_secret)
	BaseURL       string // region base, e.g. https://api.us.nylas.com or https://api.eu.nylas.com
	WebhookSecret string // application webhook signing secret (HMAC-SHA256)
	ClientID      string // Nylas application client id (hosted OAuth)
}

// Provider is a Nylas v3 implementation of emailprovider.Provider.
type Provider struct {
	client        *resty.Client
	baseURL       string
	apiKey        string
	webhookSecret string
	clientID      string
}

var _ emailprovider.Provider = (*Provider)(nil)

// New builds a Nylas Provider from explicit config.
func New(cfg Config) *Provider {
	base := cfg.BaseURL
	if base == "" {
		base = defaultBaseURL
	}
	base = strings.TrimRight(base, "/")
	client := resty.New().
		SetBaseURL(base).
		SetAuthToken(cfg.APIKey).
		SetHeader("Accept", "application/json").
		SetHeader("Content-Type", "application/json").
		SetTimeout(30 * time.Second)
	return &Provider{
		client:        client,
		baseURL:       base,
		apiKey:        cfg.APIKey,
		webhookSecret: cfg.WebhookSecret,
		clientID:      cfg.ClientID,
	}
}

// NewFromEnv builds a Nylas Provider from NYLAS_API_KEY, NYLAS_API_URI,
// NYLAS_WEBHOOK_SECRET and NYLAS_CLIENT_ID.
func NewFromEnv() (*Provider, error) {
	key := os.Getenv("NYLAS_API_KEY")
	if key == "" {
		return nil, fmt.Errorf("nylas: NYLAS_API_KEY not set")
	}
	return New(Config{
		APIKey:        key,
		BaseURL:       os.Getenv("NYLAS_API_URI"),
		WebhookSecret: os.Getenv("NYLAS_WEBHOOK_SECRET"),
		ClientID:      os.Getenv("NYLAS_CLIENT_ID"),
	}), nil
}

// HasHostedAuth reports whether hosted OAuth is configured (client id present).
func (p *Provider) HasHostedAuth() bool { return p.clientID != "" }

// Name implements emailprovider.Provider.
func (p *Provider) Name() string { return "nylas" }

// --- Nylas v3 wire types (subset) ---

type nylasAddr struct {
	Name  string `json:"name,omitempty"`
	Email string `json:"email"`
}

type nylasMessage struct {
	ID       string      `json:"id"`
	GrantID  string      `json:"grant_id"`
	ThreadID string      `json:"thread_id"`
	Subject  string      `json:"subject"`
	From     []nylasAddr `json:"from"`
	To       []nylasAddr `json:"to"`
	Cc       []nylasAddr `json:"cc"`
	Bcc      []nylasAddr `json:"bcc"`
	Snippet  string      `json:"snippet"`
	Body     string      `json:"body"`
	Date     int64       `json:"date"` // unix seconds
	Folders  []string    `json:"folders"`
}

type nylasMessageResp struct {
	Data nylasMessage `json:"data"`
}

type nylasMessagesResp struct {
	Data []nylasMessage `json:"data"`
}

type nylasSendResp struct {
	Data struct {
		ID       string `json:"id"`
		ThreadID string `json:"thread_id"`
		Date     int64  `json:"date"`
	} `json:"data"`
}

type nylasWebhookResp struct {
	Data struct {
		ID            string   `json:"id"`
		WebhookURL    string   `json:"webhook_url"`
		TriggerTypes  []string `json:"trigger_types"`
		WebhookSecret string   `json:"webhook_secret"`
	} `json:"data"`
}

// --- normalization ---

func toAddresses(in []nylasAddr) []emailprovider.Address {
	out := make([]emailprovider.Address, 0, len(in))
	for _, a := range in {
		out = append(out, emailprovider.Address{Name: a.Name, Email: a.Email})
	}
	return out
}

func firstAddress(in []nylasAddr) emailprovider.Address {
	if len(in) == 0 {
		return emailprovider.Address{}
	}
	return emailprovider.Address{Name: in[0].Name, Email: in[0].Email}
}

func fromAddresses(in []emailprovider.Address) []nylasAddr {
	out := make([]nylasAddr, 0, len(in))
	for _, a := range in {
		out = append(out, nylasAddr{Name: a.Name, Email: a.Email})
	}
	return out
}

// normalize maps a Nylas message onto the provider-neutral Message. Nylas v3
// unifies labels and folders into one `folders` list; Folder takes the first
// entry and Labels mirrors the whole list.
func (p *Provider) normalize(nm nylasMessage) emailprovider.Message {
	folder := ""
	if len(nm.Folders) > 0 {
		folder = nm.Folders[0]
	}
	labels := make([]string, len(nm.Folders))
	copy(labels, nm.Folders)
	return emailprovider.Message{
		ID:         nm.ID,
		ThreadID:   nm.ThreadID,
		Subject:    nm.Subject,
		From:       firstAddress(nm.From),
		To:         toAddresses(nm.To),
		Cc:         toAddresses(nm.Cc),
		Bcc:        toAddresses(nm.Bcc),
		Snippet:    nm.Snippet,
		Body:       nm.Body,
		ReceivedAt: time.Unix(nm.Date, 0).UTC(),
		Folder:     folder,
		Labels:     labels,
	}
}

// --- Provider methods ---

// FetchMessage implements emailprovider.Provider.
func (p *Provider) FetchMessage(ctx context.Context, grantID, messageID string) (*emailprovider.Message, error) {
	var out nylasMessageResp
	resp, err := p.client.R().
		SetContext(ctx).
		SetResult(&out).
		Get(fmt.Sprintf("/v3/grants/%s/messages/%s", grantID, messageID))
	if err != nil {
		return nil, fmt.Errorf("nylas: fetch message: %w", err)
	}
	if resp.IsError() {
		return nil, fmt.Errorf("nylas: fetch message %s: status %d", messageID, resp.StatusCode())
	}
	msg := p.normalize(out.Data)
	return &msg, nil
}

// ListThread implements emailprovider.Provider. It lists the thread's messages
// (GET /messages?thread_id=) and derives the subject from the first message.
func (p *Provider) ListThread(ctx context.Context, grantID, threadID string) (*emailprovider.Thread, error) {
	var out nylasMessagesResp
	resp, err := p.client.R().
		SetContext(ctx).
		SetQueryParam("thread_id", threadID).
		SetResult(&out).
		Get(fmt.Sprintf("/v3/grants/%s/messages", grantID))
	if err != nil {
		return nil, fmt.Errorf("nylas: list thread: %w", err)
	}
	if resp.IsError() {
		return nil, fmt.Errorf("nylas: list thread %s: status %d", threadID, resp.StatusCode())
	}
	thread := &emailprovider.Thread{ID: threadID}
	for _, nm := range out.Data {
		thread.Messages = append(thread.Messages, p.normalize(nm))
	}
	if len(thread.Messages) > 0 {
		thread.Subject = thread.Messages[0].Subject
	}
	return thread, nil
}

// Send implements emailprovider.Provider. It sends through the grant's own
// mailbox; deliverability is the user's mailbox reputation (no ESP warmup).
func (p *Provider) Send(ctx context.Context, grantID string, req emailprovider.SendRequest) (*emailprovider.SendResult, error) {
	body := map[string]any{
		"to":      fromAddresses(req.To),
		"subject": req.Subject,
		"body":    req.Body,
	}
	if len(req.Cc) > 0 {
		body["cc"] = fromAddresses(req.Cc)
	}
	if len(req.Bcc) > 0 {
		body["bcc"] = fromAddresses(req.Bcc)
	}
	if req.InReplyTo != "" {
		body["reply_to_message_id"] = req.InReplyTo
	}
	var out nylasSendResp
	resp, err := p.client.R().
		SetContext(ctx).
		SetBody(body).
		SetResult(&out).
		Post(fmt.Sprintf("/v3/grants/%s/messages/send", grantID))
	if err != nil {
		return nil, fmt.Errorf("nylas: send: %w", err)
	}
	if resp.IsError() {
		return nil, fmt.Errorf("nylas: send: status %d", resp.StatusCode())
	}
	return &emailprovider.SendResult{
		MessageID: out.Data.ID,
		ThreadID:  out.Data.ThreadID,
		SentAt:    time.Unix(out.Data.Date, 0).UTC(),
	}, nil
}

// RegisterWebhook implements emailprovider.Provider. The returned Secret is the
// application webhook signing secret; persist it for VerifySignature.
func (p *Provider) RegisterWebhook(ctx context.Context, callbackURL string, triggers []string) (*emailprovider.WebhookRegistration, error) {
	body := map[string]any{
		"trigger_types": triggers,
		"webhook_url":   callbackURL,
	}
	var out nylasWebhookResp
	resp, err := p.client.R().
		SetContext(ctx).
		SetBody(body).
		SetResult(&out).
		Post("/v3/webhooks")
	if err != nil {
		return nil, fmt.Errorf("nylas: register webhook: %w", err)
	}
	if resp.IsError() {
		return nil, fmt.Errorf("nylas: register webhook: status %d", resp.StatusCode())
	}
	return &emailprovider.WebhookRegistration{
		ID:          out.Data.ID,
		CallbackURL: out.Data.WebhookURL,
		Triggers:    out.Data.TriggerTypes,
		Secret:      out.Data.WebhookSecret,
	}, nil
}

// VerifySignature implements emailprovider.Provider: HMAC-SHA256 hex of the raw
// body keyed by the webhook secret, compared in constant time (Nylas sends the
// digest in the X-Nylas-Signature header).
func (p *Provider) VerifySignature(body []byte, signature string) bool {
	if p.webhookSecret == "" || signature == "" {
		return false
	}
	mac := hmac.New(sha256.New, []byte(p.webhookSecret))
	mac.Write(body)
	expected := hex.EncodeToString(mac.Sum(nil))
	return hmac.Equal([]byte(expected), []byte(strings.TrimSpace(signature)))
}

// --- Hosted OAuth (grant onboarding, P3b) ---

// AuthURL builds the Nylas v3 hosted-auth URL the user visits to connect a
// mailbox. state is an opaque (sealed) token echoed back to the callback;
// loginHint pre-fills the email and may be empty.
func (p *Provider) AuthURL(redirectURI, state, loginHint string) (string, error) {
	if p.clientID == "" {
		return "", fmt.Errorf("nylas: NYLAS_CLIENT_ID not set; hosted auth unavailable")
	}
	q := url.Values{}
	q.Set("client_id", p.clientID)
	q.Set("redirect_uri", redirectURI)
	q.Set("response_type", "code")
	q.Set("access_type", "offline")
	q.Set("state", state)
	if loginHint != "" {
		q.Set("login_hint", loginHint)
	}
	return fmt.Sprintf("%s/v3/connect/auth?%s", p.baseURL, q.Encode()), nil
}

type nylasTokenResp struct {
	GrantID  string `json:"grant_id"`
	Email    string `json:"email"`
	Provider string `json:"provider"`
}

// ExchangeCode swaps an authorization code for a grant (the connected mailbox).
// The API key is the client_secret in the v3 token exchange.
func (p *Provider) ExchangeCode(ctx context.Context, code, redirectURI string) (*emailprovider.Grant, error) {
	if p.clientID == "" {
		return nil, fmt.Errorf("nylas: NYLAS_CLIENT_ID not set; hosted auth unavailable")
	}
	var out nylasTokenResp
	resp, err := p.client.R().
		SetContext(ctx).
		SetBody(map[string]any{
			"client_id":     p.clientID,
			"client_secret": p.apiKey,
			"grant_type":    "authorization_code",
			"code":          code,
			"redirect_uri":  redirectURI,
		}).
		SetResult(&out).
		Post("/v3/connect/token")
	if err != nil {
		return nil, fmt.Errorf("nylas: exchange code: %w", err)
	}
	if resp.IsError() {
		return nil, fmt.Errorf("nylas: exchange code: status %d", resp.StatusCode())
	}
	if out.GrantID == "" {
		return nil, fmt.Errorf("nylas: exchange code: no grant_id in response")
	}
	return &emailprovider.Grant{ID: out.GrantID, Email: out.Email, Provider: out.Provider}, nil
}
