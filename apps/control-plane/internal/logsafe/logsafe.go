// Package logsafe sanitises untrusted values before they are written to logs.
//
// Log injection (CodeQL go/log-injection): any attacker-controlled string that
// reaches a log line can contain CR/LF and forge additional log entries. An
// attacker who can pick their own email address, OAuth provider name, or
// webhook id can therefore write arbitrary lines into our logs — enough to
// fake an audit trail, hide a real entry in noise, or break a downstream log
// parser that splits on newlines.
//
// Sanitising at the log call site (rather than at input validation) is
// deliberate: these values are legitimately allowed to contain odd characters
// elsewhere in the system, and we do not want to reject a signup because an
// address is unusual. The log line is the only place the bytes are dangerous.
package logsafe

import "strings"

// Max length for a sanitised value. Long enough for an email, a UUID, or a
// provider id; short enough that a megabyte of junk cannot flood a log file.
const maxLen = 256

// String returns v with anything that could forge a log entry removed:
// newlines, carriage returns, tabs, and other C0/C1 control characters. The
// result is truncated to a sane length and marked when truncated.
//
// Replacement (rather than deletion) keeps the value's shape visible, so a
// forged-looking entry is obvious in the log rather than silently reassembled.
func String(v string) string {
	truncated := false
	if len(v) > maxLen {
		v = v[:maxLen]
		truncated = true
	}

	var b strings.Builder
	b.Grow(len(v))
	for _, r := range v {
		switch {
		case r == '\n' || r == '\r':
			// The characters that actually forge a new log entry.
			b.WriteString("\\n")
		case r == '\t':
			b.WriteString("\\t")
		case r < 0x20 || r == 0x7f:
			// Remaining C0 controls + DEL: terminal escapes, NUL, etc.
			b.WriteByte('?')
		default:
			b.WriteRune(r)
		}
	}

	out := b.String()
	if truncated {
		out += "…(truncated)"
	}
	return out
}
