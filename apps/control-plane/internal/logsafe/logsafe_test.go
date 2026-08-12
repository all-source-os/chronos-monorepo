package logsafe

import (
	"strings"
	"testing"
)

func TestStripsNewlinesThatWouldForgeALogEntry(t *testing.T) {
	// The actual attack: an attacker-chosen email that appends a fake line.
	got := String("victim@example.com\n2026-08-11 ADMIN promoted attacker@evil.test")
	if strings.ContainsAny(got, "\n\r") {
		t.Fatalf("sanitized value still contains a line break: %q", got)
	}
	if !strings.Contains(got, `\n`) {
		t.Fatalf("expected the break to be made visible as \\n, got %q", got)
	}
}

func TestStripsCarriageReturnAndControlCharacters(t *testing.T) {
	got := String("a\rb\x00c\x1b[31md\x7fe")
	if strings.ContainsAny(got, "\r\x00\x1b\x7f") {
		t.Fatalf("control characters survived: %q", got)
	}
}

func TestKeepsOrdinaryValuesIntact(t *testing.T) {
	for _, v := range []string{
		"user@example.com",
		"google",
		"tenant-abc-123",
		"Ünïcødé is fine",
	} {
		if got := String(v); got != v {
			t.Errorf("String(%q) = %q, want it unchanged", v, got)
		}
	}
}

func TestTruncatesOverlongValues(t *testing.T) {
	got := String(strings.Repeat("x", maxLen*3))
	if len(got) > maxLen+len("…(truncated)") {
		t.Fatalf("value not truncated: len=%d", len(got))
	}
	if !strings.HasSuffix(got, "…(truncated)") {
		t.Fatalf("truncation not signaled: %q", got[len(got)-20:])
	}
}

func TestEmptyStringIsSafe(t *testing.T) {
	if got := String(""); got != "" {
		t.Fatalf("String(\"\") = %q, want empty", got)
	}
}
