package nylas

import (
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"testing"

	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider"
)

func TestFetchMessage_Normalizes(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if got := r.URL.Path; got != "/v3/grants/grant_x/messages/msg_1" {
			t.Errorf("unexpected path %q", got)
		}
		if auth := r.Header.Get("Authorization"); auth != "Bearer key_test" {
			t.Errorf("missing bearer auth, got %q", auth)
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = io.WriteString(w, `{"request_id":"rq","data":{
			"id":"msg_1","grant_id":"grant_x","thread_id":"thr_1","subject":"Hi",
			"from":[{"name":"Dana","email":"dana@acme.com"}],
			"to":[{"email":"me@all-source.xyz"}],
			"snippet":"hello","body":"hello there","date":1750000000,"folders":["inbox"]}}`)
	}))
	defer srv.Close()

	p := New(Config{APIKey: "key_test", BaseURL: srv.URL})
	msg, err := p.FetchMessage(context.Background(), "grant_x", "msg_1")
	if err != nil {
		t.Fatalf("FetchMessage: %v", err)
	}
	if msg.ID != "msg_1" || msg.ThreadID != "thr_1" || msg.Subject != "Hi" {
		t.Errorf("bad ids/subject: %+v", msg)
	}
	if msg.From.Name != "Dana" || msg.From.Email != "dana@acme.com" {
		t.Errorf("bad from: %+v", msg.From)
	}
	if len(msg.To) != 1 || msg.To[0].Email != "me@all-source.xyz" {
		t.Errorf("bad to: %+v", msg.To)
	}
	if msg.Folder != "inbox" || len(msg.Labels) != 1 || msg.Labels[0] != "inbox" {
		t.Errorf("bad folder/labels: %q %v", msg.Folder, msg.Labels)
	}
	if got := msg.ReceivedAt.UTC().Format("2006-01-02T15:04:05Z"); got == "" {
		t.Errorf("received_at not set")
	}
}

func TestSend_BuildsReplyBody(t *testing.T) {
	var gotBody map[string]any
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v3/grants/grant_x/messages/send" || r.Method != http.MethodPost {
			t.Errorf("unexpected %s %s", r.Method, r.URL.Path)
		}
		raw, _ := io.ReadAll(r.Body)
		if err := json.Unmarshal(raw, &gotBody); err != nil {
			t.Fatalf("decode body: %v", err)
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = io.WriteString(w, `{"data":{"id":"msg_sent","thread_id":"thr_1","date":1750000100}}`)
	}))
	defer srv.Close()

	p := New(Config{APIKey: "k", BaseURL: srv.URL})
	res, err := p.Send(context.Background(), "grant_x", emailprovider.SendRequest{
		Subject:   "Re: Hi",
		Body:      "thanks",
		InReplyTo: "msg_1",
		To:        []emailprovider.Address{{Email: "dana@acme.com"}},
	})
	if err != nil {
		t.Fatalf("Send: %v", err)
	}
	if res.MessageID != "msg_sent" || res.ThreadID != "thr_1" {
		t.Errorf("bad send result: %+v", res)
	}
	if gotBody["reply_to_message_id"] != "msg_1" {
		t.Errorf("reply_to_message_id not forwarded: %v", gotBody["reply_to_message_id"])
	}
}

func TestVerifySignature(t *testing.T) {
	secret := "whsec_123"
	body := []byte(`{"deltas":[]}`)
	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write(body)
	good := hex.EncodeToString(mac.Sum(nil))

	p := New(Config{APIKey: "k", WebhookSecret: secret})
	if !p.VerifySignature(body, good) {
		t.Error("valid signature rejected")
	}
	if p.VerifySignature(body, "deadbeef") {
		t.Error("invalid signature accepted")
	}
	if p.VerifySignature([]byte("tampered"), good) {
		t.Error("signature accepted for tampered body")
	}
	noSecret := New(Config{APIKey: "k"})
	if noSecret.VerifySignature(body, good) {
		t.Error("signature accepted with no configured secret")
	}
}

// TestToReceivedPayload_MatchesContract guards drift between the connector's
// normalized output and the committed email.received contract: the payload key
// set the connector emits must equal the fixture's payload key set.
func TestToReceivedPayload_MatchesContract(t *testing.T) {
	p := New(Config{APIKey: "k"})
	msg := p.normalize(nylasMessage{
		ID: "msg_0af31c8e", ThreadID: "thr_92ab17", Subject: "Re: Q3 renewal",
		From:    []nylasAddr{{Name: "Dana Lee", Email: "dana@acme.com"}},
		To:      []nylasAddr{{Name: "Founder", Email: "founder@all-source.xyz"}},
		Snippet: "Following up on the renewal terms…", Body: "body", Date: 1750000000,
		Folders: []string{"inbox"},
	})
	raw, err := json.Marshal(msg.ToReceivedPayload())
	if err != nil {
		t.Fatalf("marshal payload: %v", err)
	}
	var connKeys map[string]json.RawMessage
	if err := json.Unmarshal(raw, &connKeys); err != nil {
		t.Fatalf("unmarshal connector payload: %v", err)
	}

	fixture := filepath.Join(repoRoot(t), "docs/contracts/email-events/examples/email.received.json")
	fb, err := os.ReadFile(fixture)
	if err != nil {
		t.Fatalf("read fixture: %v", err)
	}
	var ev struct {
		Payload map[string]json.RawMessage `json:"payload"`
	}
	if err := json.Unmarshal(fb, &ev); err != nil {
		t.Fatalf("unmarshal fixture: %v", err)
	}

	if c, f := keys(connKeys), keys(ev.Payload); !equalStrings(c, f) {
		t.Errorf("payload key drift:\n connector=%v\n contract =%v", c, f)
	}
}

func keys(m map[string]json.RawMessage) []string {
	out := make([]string, 0, len(m))
	for k := range m {
		out = append(out, k)
	}
	sort.Strings(out)
	return out
}

func equalStrings(a, b []string) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

func repoRoot(t *testing.T) string {
	t.Helper()
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	dir := filepath.Dir(file)
	for range 12 {
		if _, err := os.Stat(filepath.Join(dir, "docs", "contracts", "email-events")); err == nil {
			return dir
		}
		dir = filepath.Dir(dir)
	}
	t.Fatal("repo root (docs/contracts/email-events) not found")
	return ""
}
