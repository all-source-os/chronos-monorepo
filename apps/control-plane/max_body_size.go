package main

import (
	"fmt"
	"net/http"

	"github.com/gin-gonic/gin"
)

// MaxBodySize returns a Gin middleware that rejects requests with a body
// exceeding the given limit in bytes. Returns 413 Payload Too Large with
// a clear error message including the limit.
//
// Apply this to ingest routes to prevent storage abuse via oversized events.
func MaxBodySize(maxBytes int64) gin.HandlerFunc {
	return func(c *gin.Context) {
		if c.Request.ContentLength > maxBytes {
			c.JSON(http.StatusRequestEntityTooLarge, gin.H{
				"error":   "payload_too_large",
				"message": fmt.Sprintf("Request body exceeds %s limit", formatBytes(maxBytes)),
				"limit":   maxBytes,
			})
			c.Abort()
			return
		}

		// Also enforce via http.MaxBytesReader to catch chunked transfers
		// where Content-Length may be absent or inaccurate.
		c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, maxBytes)
		c.Next()

		// Check if MaxBytesReader tripped during read
		if c.IsAborted() {
			return
		}
	}
}

func formatBytes(b int64) string {
	switch {
	case b >= 1<<20:
		return fmt.Sprintf("%dMB", b/(1<<20))
	case b >= 1<<10:
		return fmt.Sprintf("%dKB", b/(1<<10))
	default:
		return fmt.Sprintf("%d bytes", b)
	}
}
