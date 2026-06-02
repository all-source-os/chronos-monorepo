//! MCP stdio transport — newline-delimited JSON-RPC (per MCP spec).
//!
//! The MCP stdio transport frames messages as one JSON-RPC object per line:
//! no headers, no embedded newlines. Responses are written compact and
//! `\n`-terminated. Inbound parsing also tolerates legacy `Content-Length:`
//! (LSP-style) framing so older callers keep working, but spec-compliant
//! clients (e.g. Claude Code) only ever see line-delimited output.

use allsource_core::prime::{Prime, recall::RecallEngine};
use anyhow::Result;
use std::{
    io::{BufRead, Write},
    sync::Arc,
};

use crate::{
    dispatch,
    protocol::{Request, Response},
};

pub struct StdioTransport {
    prime: Arc<Prime>,
    recall: RecallEngine,
    auto_inject: bool,
    auto_inject_max_tokens: usize,
}

impl StdioTransport {
    pub fn new(prime: Arc<Prime>, recall: RecallEngine) -> Self {
        Self {
            prime,
            recall,
            auto_inject: false,
            auto_inject_max_tokens: 1000,
        }
    }

    pub fn with_auto_inject(mut self, max_tokens: usize) -> Self {
        self.auto_inject = true;
        self.auto_inject_max_tokens = max_tokens;
        self
    }

    pub async fn run(&mut self) -> Result<()> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let mut reader = std::io::BufReader::new(stdin.lock());

        loop {
            let Some(body) = read_message(&mut reader)? else {
                break; // EOF
            };

            let body = body.trim().to_string();
            if body.is_empty() {
                continue;
            }

            tracing::debug!("recv: {body}");

            let request: Request = match serde_json::from_str(&body) {
                Ok(r) => r,
                Err(e) => {
                    let resp = Response::error(None, -32700, format!("Parse error: {e}"));
                    write_response(&mut stdout, &resp)?;
                    continue;
                }
            };

            let response = self.handle_request(&request).await;

            if let Some(resp) = response {
                write_response(&mut stdout, &resp)?;
            }
        }

        tracing::info!("stdin closed, shutting down");
        Ok(())
    }

    async fn handle_request(&self, req: &Request) -> Option<Response> {
        dispatch::handle_request(
            &self.prime,
            &self.recall,
            self.auto_inject,
            self.auto_inject_max_tokens,
            req,
        )
        .await
    }
}

/// Read a single MCP message from the reader.
///
/// Supports two modes:
/// 1. **Newline-delimited JSON** (MCP spec): if the first line starts with `{`,
///    it is one complete JSON-RPC object.
/// 2. **Content-Length framing** (legacy LSP-style): headers ending with a
///    blank line, then exact body bytes — accepted for backward compatibility.
fn read_message(reader: &mut impl BufRead) -> Result<Option<String>> {
    let mut first_line = String::new();
    let bytes_read = reader.read_line(&mut first_line)?;
    if bytes_read == 0 {
        return Ok(None); // EOF
    }

    let trimmed = first_line.trim();

    // Fallback: if the line starts with `{`, it's line-delimited JSON (backward compat)
    if trimmed.starts_with('{') {
        return Ok(Some(trimmed.to_string()));
    }

    // Content-Length framing: parse headers
    let content_length = if let Some(value) = trimmed.strip_prefix("Content-Length:") {
        value
            .trim()
            .parse::<usize>()
            .map_err(|e| anyhow::anyhow!("invalid Content-Length: {e}"))?
    } else {
        // Unknown header line — skip until we find Content-Length or empty line
        return read_message(reader); // recurse to find next message
    };

    // Read remaining headers until blank line
    loop {
        let mut header = String::new();
        let n = reader.read_line(&mut header)?;
        if n == 0 {
            return Ok(None); // EOF
        }
        if header.trim().is_empty() {
            break; // End of headers
        }
        // Ignore other headers (Content-Type, etc.)
    }

    // Read exact body
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;

    Ok(Some(String::from_utf8_lossy(&body).to_string()))
}

/// Write a response as newline-delimited JSON-RPC (per the MCP stdio spec).
///
/// `serde_json::to_string` produces compact output with no embedded newlines,
/// so a single trailing `\n` cleanly delimits the message for line-based
/// clients.
fn write_response(writer: &mut impl Write, resp: &Response) -> Result<()> {
    let json = serde_json::to_string(resp)?;
    tracing::debug!("send: {json}");
    writeln!(writer, "{json}")?;
    writer.flush()?;
    Ok(())
}
