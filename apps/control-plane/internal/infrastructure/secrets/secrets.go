// Package secrets does authenticated symmetric encryption (AES-256-GCM) of small
// connector secrets — the per-grant records the AI inbox writes to Core config.
// Core config is plaintext KV, so a grant record must be sealed before SetConfig
// (docs/proposals/AI_INBOX_ON_ALLSOURCE.md §7: a plaintext grant must never reach
// Core config in prod). The key comes from CONNECTOR_SECRET_KEY (base64 32 bytes).
package secrets

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"encoding/base64"
	"errors"
	"fmt"
	"io"
	"os"
	"strings"
)

// sealPrefix tags sealed values so Open can distinguish ciphertext from a legacy
// plaintext value written before encryption existed.
const sealPrefix = "enc:v1:"

// ErrNotSealed means the token lacks the seal prefix (e.g. a legacy plaintext
// value). Callers may fall back to treating it as plaintext during migration.
var ErrNotSealed = errors.New("secrets: value is not sealed")

// Sealer seals/opens connector secrets with a fixed AES-256-GCM key.
type Sealer struct {
	gcm cipher.AEAD
}

// NewSealer builds a Sealer from a 32-byte key.
func NewSealer(key []byte) (*Sealer, error) {
	if len(key) != 32 {
		return nil, fmt.Errorf("secrets: key must be 32 bytes, got %d", len(key))
	}
	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}
	return &Sealer{gcm: gcm}, nil
}

// NewSealerFromEnv builds a Sealer from CONNECTOR_SECRET_KEY (base64 32 bytes).
func NewSealerFromEnv() (*Sealer, error) {
	raw := os.Getenv("CONNECTOR_SECRET_KEY")
	if raw == "" {
		return nil, errors.New("secrets: CONNECTOR_SECRET_KEY not set")
	}
	key, err := base64.StdEncoding.DecodeString(strings.TrimSpace(raw))
	if err != nil {
		return nil, fmt.Errorf("secrets: CONNECTOR_SECRET_KEY not valid base64: %w", err)
	}
	return NewSealer(key)
}

// Seal returns sealPrefix + base64(nonce||ciphertext||tag).
func (s *Sealer) Seal(plaintext []byte) (string, error) {
	nonce := make([]byte, s.gcm.NonceSize())
	if _, err := io.ReadFull(rand.Reader, nonce); err != nil {
		return "", fmt.Errorf("secrets: read nonce: %w", err)
	}
	sealed := s.gcm.Seal(nonce, nonce, plaintext, nil)
	return sealPrefix + base64.StdEncoding.EncodeToString(sealed), nil
}

// Open reverses Seal. Returns ErrNotSealed if the token is not a sealed value.
func (s *Sealer) Open(token string) ([]byte, error) {
	rest, ok := strings.CutPrefix(token, sealPrefix)
	if !ok {
		return nil, ErrNotSealed
	}
	raw, err := base64.StdEncoding.DecodeString(rest)
	if err != nil {
		return nil, fmt.Errorf("secrets: decode sealed value: %w", err)
	}
	ns := s.gcm.NonceSize()
	if len(raw) < ns {
		return nil, errors.New("secrets: sealed value too short")
	}
	nonce, ciphertext := raw[:ns], raw[ns:]
	plaintext, err := s.gcm.Open(nil, nonce, ciphertext, nil)
	if err != nil {
		return nil, fmt.Errorf("secrets: open: %w", err)
	}
	return plaintext, nil
}

// IsSealed reports whether a stored value is a sealed token.
func IsSealed(token string) bool {
	return strings.HasPrefix(token, sealPrefix)
}
