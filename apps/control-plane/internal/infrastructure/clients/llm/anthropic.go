// Package llm provides a thin Anthropic Messages API client for server-side
// generation — currently AI inbox draft generation (045). It follows the
// resty-typed-client pattern used across the Control Plane infra layer
// (CoreClient, nylas.Provider) rather than pulling in the full Anthropic SDK.
// The request/response wire shape is the documented Messages API
// (POST https://api.anthropic.com/v1/messages; x-api-key + anthropic-version).
package llm

import (
	"context"
	"errors"
	"fmt"
	"os"
	"strings"
	"time"

	"github.com/go-resty/resty/v2"
)

const (
	defaultBaseURL   = "https://api.anthropic.com"
	anthropicVersion = "2023-06-01"
	// defaultModel — the latest Opus. Adaptive thinking + the effort parameter
	// are GA on this model (no beta header). Override with ANTHROPIC_MODEL.
	defaultModel   = "claude-opus-4-8"
	defaultTimeout = 60 * time.Second
	// maxTokens caps the whole completion (adaptive thinking + the reply body).
	// A reply body is short; this leaves ample room for thinking without risking
	// the non-streaming HTTP timeout.
	maxTokens = 4096
)

// ErrRefused is returned when the model declines the request (safety classifiers
// or a content refusal — HTTP 200 with stop_reason "refusal"). Callers surface it
// rather than retry the same prompt.
var ErrRefused = errors.New("llm: request refused")

// Client talks to the Anthropic Messages API. Construct with NewFromEnv; a nil
// *Client means no key is configured and callers should gate the feature off.
type Client struct {
	client *resty.Client
	model  string
}

// NewFromEnv builds a Client from ANTHROPIC_API_KEY (plus optional ANTHROPIC_MODEL
// and ANTHROPIC_BASE_URL). It returns nil when ANTHROPIC_API_KEY is unset — draft
// generation is an optional feature; the inbox handler returns 503 when nil.
func NewFromEnv() *Client {
	key := os.Getenv("ANTHROPIC_API_KEY")
	if key == "" {
		return nil
	}
	base := os.Getenv("ANTHROPIC_BASE_URL")
	if base == "" {
		base = defaultBaseURL
	}
	model := os.Getenv("ANTHROPIC_MODEL")
	if model == "" {
		model = defaultModel
	}
	client := resty.New().
		SetBaseURL(strings.TrimRight(base, "/")).
		SetHeader("x-api-key", key).
		SetHeader("anthropic-version", anthropicVersion).
		SetHeader("content-type", "application/json").
		SetTimeout(defaultTimeout)
	return &Client{client: client, model: model}
}

// --- Messages API wire types (subset) ---

type messageRequest struct {
	Model        string         `json:"model"`
	MaxTokens    int            `json:"max_tokens"`
	System       string         `json:"system,omitempty"`
	Thinking     map[string]any `json:"thinking,omitempty"`
	OutputConfig map[string]any `json:"output_config,omitempty"`
	Messages     []wireMessage  `json:"messages"`
}

type wireMessage struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

type contentBlock struct {
	Type string `json:"type"`
	Text string `json:"text"`
}

type messageResponse struct {
	Content    []contentBlock `json:"content"`
	StopReason string         `json:"stop_reason"`
}

// GenerateReply runs a single non-streaming completion: `system` frames the task,
// `user` carries the grounded prompt. It returns the concatenated text blocks.
// Adaptive thinking is enabled (recommended on Opus 4.8) with medium effort — a
// good cost/quality balance for short, grounded drafting. Returns ErrRefused when
// the model declines.
func (c *Client) GenerateReply(ctx context.Context, system, user string) (string, error) {
	body := messageRequest{
		Model:        c.model,
		MaxTokens:    maxTokens,
		System:       system,
		Thinking:     map[string]any{"type": "adaptive"},
		OutputConfig: map[string]any{"effort": "medium"},
		Messages:     []wireMessage{{Role: "user", Content: user}},
	}
	var out messageResponse
	resp, err := c.client.R().
		SetContext(ctx).
		SetBody(body).
		SetResult(&out).
		Post("/v1/messages")
	if err != nil {
		return "", fmt.Errorf("llm: request: %w", err)
	}
	if resp.IsError() {
		return "", fmt.Errorf("llm: status %d: %s", resp.StatusCode(), resp.String())
	}
	// A safety/content refusal is a 200 with stop_reason "refusal" and (usually)
	// empty content — check before reading text.
	if out.StopReason == "refusal" {
		return "", ErrRefused
	}
	var sb strings.Builder
	for _, b := range out.Content {
		if b.Type == "text" {
			sb.WriteString(b.Text)
		}
	}
	text := strings.TrimSpace(sb.String())
	if text == "" {
		return "", fmt.Errorf("llm: empty completion (stop_reason=%q)", out.StopReason)
	}
	return text, nil
}
