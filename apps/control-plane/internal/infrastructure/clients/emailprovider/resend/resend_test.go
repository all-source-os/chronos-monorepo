package resend

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"net/http"
	"strconv"
	"testing"
	"time"
)

func sign(key []byte, id, ts string, body []byte) string {
	mac := hmac.New(sha256.New, key)
	mac.Write([]byte(id + "." + ts + "." + string(body)))
	return "v1," + base64.StdEncoding.EncodeToString(mac.Sum(nil))
}

func TestVerifyWebhook(t *testing.T) {
	key := []byte("0123456789abcdef0123456789abcdef")
	secret := "whsec_" + base64.StdEncoding.EncodeToString(key)
	p := New("re_x", secret)

	body := []byte(`{"type":"email.received","data":{"email_id":"em1"}}`)
	id := "msg_123"
	ts := strconv.FormatInt(time.Now().Unix(), 10)
	hdr := func(sig string) http.Header {
		h := http.Header{}
		h.Set("svix-id", id)
		h.Set("svix-timestamp", ts)
		h.Set("svix-signature", sig)
		return h
	}

	if !p.VerifyWebhook(hdr(sign(key, id, ts, body)), body) {
		t.Fatal("valid signature rejected")
	}
	if p.VerifyWebhook(hdr(sign(key, id, ts, body)), []byte(`{"tampered":true}`)) {
		t.Error("tampered body accepted")
	}
	if p.VerifyWebhook(http.Header{}, body) {
		t.Error("missing headers accepted")
	}
	if p.VerifyWebhook(hdr("v1,not-a-real-signature"), body) {
		t.Error("wrong signature accepted")
	}
	// Expired timestamp (outside the replay window).
	old := strconv.FormatInt(time.Now().Add(-10*time.Minute).Unix(), 10)
	h := http.Header{}
	h.Set("svix-id", id)
	h.Set("svix-timestamp", old)
	h.Set("svix-signature", sign(key, id, old, body))
	if p.VerifyWebhook(h, body) {
		t.Error("expired timestamp accepted")
	}
	// No secret → fail closed.
	if New("re_x", "").VerifyWebhook(hdr(sign(key, id, ts, body)), body) {
		t.Error("verify without a configured secret accepted")
	}
}

func TestNormalize(t *testing.T) {
	m := normalize(receivedMessage{
		ID:        "em1",
		MessageID: "<root@x>",
		From:      "Dana <dana@acme.com>",
		To:        []string{"sales@all-source.xyz"},
		Subject:   "Renewal",
		Text:      "Can we renew?",
		Headers:   map[string]string{"References": "<root@x> <reply@x>"},
		CreatedAt: "2026-06-20T10:00:00Z",
	})
	if m.ID != "<root@x>" {
		t.Errorf("entity id should be the RFC Message-ID, got %q", m.ID)
	}
	if m.ThreadID != "<root@x>" {
		t.Errorf("thread id should be the References root, got %q", m.ThreadID)
	}
	if m.From.Email != "dana@acme.com" || m.From.Name != "Dana" {
		t.Errorf("bad From: %+v", m.From)
	}
	if m.Body != "Can we renew?" {
		t.Errorf("bad Body: %q", m.Body)
	}
}

func TestDeriveThreadIDFallbacks(t *testing.T) {
	// In-Reply-To when no References.
	if got := deriveThreadID(map[string]string{"In-Reply-To": "<a@x>"}, "<self@x>"); got != "<a@x>" {
		t.Errorf("want In-Reply-To, got %q", got)
	}
	// Self id when neither header present (a new thread).
	if got := deriveThreadID(nil, "<self@x>"); got != "<self@x>" {
		t.Errorf("want self id, got %q", got)
	}
}
