package secrets

import (
	"bytes"
	"errors"
	"testing"
)

func key32() []byte {
	k := make([]byte, 32)
	for i := range k {
		k[i] = byte(i)
	}
	return k
}

func TestSealOpenRoundtrip(t *testing.T) {
	s, err := NewSealer(key32())
	if err != nil {
		t.Fatalf("NewSealer: %v", err)
	}
	plain := []byte(`{"tenant_id":"tnt1","grant_id":"g1"}`)
	token, err := s.Seal(plain)
	if err != nil {
		t.Fatalf("Seal: %v", err)
	}
	if !IsSealed(token) {
		t.Fatalf("token not marked sealed: %q", token)
	}
	got, err := s.Open(token)
	if err != nil {
		t.Fatalf("Open: %v", err)
	}
	if !bytes.Equal(got, plain) {
		t.Errorf("roundtrip mismatch: %s", got)
	}
}

func TestSealIsNondeterministic(t *testing.T) {
	s, err := NewSealer(key32())
	if err != nil {
		t.Fatalf("NewSealer: %v", err)
	}
	a, err := s.Seal([]byte("x"))
	if err != nil {
		t.Fatalf("Seal: %v", err)
	}
	b, err := s.Seal([]byte("x"))
	if err != nil {
		t.Fatalf("Seal: %v", err)
	}
	if a == b {
		t.Error("two seals of the same plaintext are identical (nonce reuse?)")
	}
}

func TestOpenRejectsLegacyPlaintext(t *testing.T) {
	s, _ := NewSealer(key32()) //nolint:errcheck // test plumbing
	_, err := s.Open("tnt1")   // a legacy plaintext tenant id, not sealed
	if !errors.Is(err, ErrNotSealed) {
		t.Errorf("want ErrNotSealed, got %v", err)
	}
}

func TestOpenFailsWithWrongKey(t *testing.T) {
	s1, err := NewSealer(key32())
	if err != nil {
		t.Fatalf("NewSealer: %v", err)
	}
	token, err := s1.Seal([]byte("secret"))
	if err != nil {
		t.Fatalf("Seal: %v", err)
	}

	other := key32()
	other[0] ^= 0xFF
	s2, _ := NewSealer(other) //nolint:errcheck // test plumbing
	if _, err := s2.Open(token); err == nil {
		t.Error("opened with wrong key — auth tag not enforced")
	}
}

func TestNewSealerRejectsBadKeyLength(t *testing.T) {
	if _, err := NewSealer([]byte("short")); err == nil {
		t.Error("accepted a non-32-byte key")
	}
}
