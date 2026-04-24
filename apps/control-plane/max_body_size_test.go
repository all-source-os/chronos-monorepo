package main

import (
	"context"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/gin-gonic/gin"
)

func TestMaxBodySize_AllowsUnderLimit(t *testing.T) {
	gin.SetMode(gin.TestMode)
	router := gin.New()
	router.Use(MaxBodySize(1024)) // 1KB
	router.POST("/test", func(c *gin.Context) { c.JSON(200, gin.H{"ok": true}) })

	body := strings.NewReader(strings.Repeat("x", 500))
	req := httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/test", body)
	req.Header.Set("Content-Length", "500")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("want 200, got %d", w.Code)
	}
}

func TestMaxBodySize_RejectsOverLimit(t *testing.T) {
	gin.SetMode(gin.TestMode)
	router := gin.New()
	router.Use(MaxBodySize(256)) // 256 bytes
	router.POST("/test", func(c *gin.Context) { c.JSON(200, gin.H{"ok": true}) })

	body := strings.NewReader(strings.Repeat("x", 500))
	req := httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/test", body)
	req.Header.Set("Content-Length", "500")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusRequestEntityTooLarge {
		t.Errorf("want 413, got %d", w.Code)
	}
}

func TestMaxBodySize_ExactLimit(t *testing.T) {
	gin.SetMode(gin.TestMode)
	router := gin.New()
	router.Use(MaxBodySize(100))
	router.POST("/test", func(c *gin.Context) { c.JSON(200, gin.H{"ok": true}) })

	body := strings.NewReader(strings.Repeat("x", 100))
	req := httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/test", body)
	req.Header.Set("Content-Length", "100")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("exact limit should pass: want 200, got %d", w.Code)
	}
}

func TestFormatBytes(t *testing.T) {
	tests := []struct {
		input int64
		want  string
	}{
		{256, "256 bytes"},
		{1024, "1KB"},
		{256 * 1024, "256KB"},
		{1024 * 1024, "1MB"},
	}
	for _, tt := range tests {
		got := formatBytes(tt.input)
		if got != tt.want {
			t.Errorf("formatBytes(%d) = %q, want %q", tt.input, got, tt.want)
		}
	}
}
