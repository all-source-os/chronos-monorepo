// Package emailprovider defines the provider-agnostic seam for the AI inbox
// connector: a single Provider interface that normalizes any email backend
// (Nylas first) into provider-neutral types. Nothing downstream of the Control
// Plane webhook knows the provider name — Core events and Prime nodes are
// provider-neutral. The normalized types map onto the durable email.* event
// contract in docs/contracts/email-events; see also
// docs/proposals/AI_INBOX_ON_ALLSOURCE.md §3.1.
package emailprovider

import (
	"context"
	"time"
)

// Address is a normalized email participant.
type Address struct {
	Name  string `json:"name,omitempty"`
	Email string `json:"email"`
}

// Message is a provider-neutral email message (inbound or stored). Message.ID is
// the provider message id and becomes the Core event entity_id.
type Message struct {
	ID         string
	ThreadID   string
	Subject    string
	From       Address
	To         []Address
	Cc         []Address
	Bcc        []Address
	Snippet    string
	Body       string // full body, inline (Core has no blob store)
	ReceivedAt time.Time
	Folder     string
	Labels     []string
}

// Thread is a normalized conversation: its messages in provider order.
type Thread struct {
	ID       string
	Subject  string
	Messages []Message
}

// SendRequest is a provider-neutral outbound message.
type SendRequest struct {
	ThreadID  string
	InReplyTo string // provider message id being replied to; empty for a new thread
	Subject   string
	To        []Address
	Cc        []Address
	Bcc       []Address
	Body      string
}

// SendResult is the outcome of a successful send.
type SendResult struct {
	MessageID string
	ThreadID  string
	SentAt    time.Time
}

// WebhookRegistration is a registered provider webhook. Secret is the signing
// secret the provider returns at creation time; store it to verify deliveries.
type WebhookRegistration struct {
	ID          string
	CallbackURL string
	Triggers    []string
	Secret      string
}

// Grant is a connected mailbox returned by the hosted-OAuth code exchange. ID is
// the provider grant id; the OAuth tokens themselves stay with the provider.
type Grant struct {
	ID       string
	Email    string
	Provider string
}

// Provider is the provider-agnostic connector interface. Nylas is the first
// implementation; a DIY Gmail/Graph path or a second bidirectional-sync vendor
// are drop-ins behind the same interface.
type Provider interface {
	// Name is the provider id stamped into event metadata (e.g. "nylas").
	Name() string
	// FetchMessage retrieves a full message (the thin webhook carries only ids).
	FetchMessage(ctx context.Context, grantID, messageID string) (*Message, error)
	// ListThread returns a conversation and its messages.
	ListThread(ctx context.Context, grantID, threadID string) (*Thread, error)
	// Send sends a message through the grant's own mailbox.
	Send(ctx context.Context, grantID string, req SendRequest) (*SendResult, error)
	// RegisterWebhook subscribes callbackURL to the given trigger types.
	RegisterWebhook(ctx context.Context, callbackURL string, triggers []string) (*WebhookRegistration, error)
	// VerifySignature reports whether body carries a valid provider HMAC signature.
	VerifySignature(body []byte, signature string) bool
}

// ReceivedPayload is the payload of an email.received Core event
// (docs/contracts/email-events/schema/email.received.schema.json). It is
// provider-neutral; the Control Plane webhook wraps it in the event envelope and
// injects tenant_id.
type ReceivedPayload struct {
	ThreadID   string    `json:"thread_id"`
	Subject    string    `json:"subject"`
	From       Address   `json:"from"`
	To         []Address `json:"to"`
	Cc         []Address `json:"cc"`
	Snippet    string    `json:"snippet"`
	Body       string    `json:"body"`
	ReceivedAt string    `json:"received_at"`
	Folder     string    `json:"folder"`
	Labels     []string  `json:"labels"`
}

// ToReceivedPayload maps a normalized Message onto the email.received contract
// payload. Empty cc/to/labels serialize as [] (not null) to match the contract;
// received_at is RFC3339 UTC.
func (m Message) ToReceivedPayload() ReceivedPayload {
	to := m.To
	if to == nil {
		to = []Address{}
	}
	cc := m.Cc
	if cc == nil {
		cc = []Address{}
	}
	labels := m.Labels
	if labels == nil {
		labels = []string{}
	}
	return ReceivedPayload{
		ThreadID:   m.ThreadID,
		Subject:    m.Subject,
		From:       m.From,
		To:         to,
		Cc:         cc,
		Snippet:    m.Snippet,
		Body:       m.Body,
		ReceivedAt: m.ReceivedAt.UTC().Format(time.RFC3339),
		Folder:     m.Folder,
		Labels:     labels,
	}
}
