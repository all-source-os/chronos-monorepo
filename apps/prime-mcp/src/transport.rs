//! MCP stdio transport with Content-Length header framing (per MCP spec).
//!
//! Reads JSON-RPC messages from stdin using `Content-Length: N\r\n\r\n<body>` framing
//! (identical to the Language Server Protocol). Falls back to line-delimited JSON
//! if the first line looks like JSON (for backward compatibility with simple tests).

use allsource_core::prime::{Prime, recall::RecallEngine};
use anyhow::Result;
use std::io::{BufRead, Write};

use crate::{
    protocol::{self, Request, Response},
    tools,
};

/// Agent-facing cookbook: usage patterns for Prime tools.
const COOKBOOK: &str = "\
# Prime Cookbook

## Pattern: Store new knowledge
1. `prime_add_node` — create entity with domain tag
2. `prime_embed` — store embedding for semantic search
3. `prime_add_edge` — connect to existing nodes (cross-domain edges are most valuable)

## Pattern: Answer a cross-domain question
1. `prime_index` — get compressed knowledge map (shows domain connections)
2. `prime_context` — retrieve facts from relevant domains
3. Combine index cross-references with retrieved facts in your answer

## Pattern: Answer a single-domain question
1. `prime_recall` — semantic search with graph expansion
2. `prime_neighbors` — explore around the best match if needed

## Pattern: Track what changed
1. `prime_history` — full audit trail for any entity
2. `prime_stats` — overall graph state

## Pattern: Correct wrong knowledge
1. `prime_forget` — soft-delete the wrong fact (preserved in history)
2. `prime_add_node` — store the correct fact
3. `prime_add_edge` — reconnect relationships

## Anti-patterns
- DON'T embed without creating a node first (orphaned vectors can't be traversed)
- DON'T use prime_search for semantic queries (use prime_recall — it uses embeddings)
- DON'T skip the domain tag on nodes (disables cross-domain reasoning)
- DON'T forget without checking prime_history first (you might lose the only copy)
- DON'T call prime_recall without an embedding vector (it requires one)

## Token budget
- prime_index: ~100-500 tokens (scales with knowledge base)
- prime_context: configurable via max_tokens (default: uncapped)
- prime_stats: ~50 tokens
- prime_neighbors: ~100 tokens per node returned
";

pub struct StdioTransport {
    prime: Prime,
    recall: RecallEngine,
    auto_inject: bool,
    auto_inject_max_tokens: usize,
}

impl StdioTransport {
    pub fn new(prime: Prime, recall: RecallEngine) -> Self {
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
        match req.method.as_str() {
            "initialize" => Some(Response::success(
                req.id.clone(),
                protocol::server_info(self.auto_inject),
            )),
            "notifications/initialized" => None,

            "tools/list" => {
                let defs = tools::tool_definitions();
                Some(Response::success(
                    req.id.clone(),
                    serde_json::json!({ "tools": defs }),
                ))
            }

            "tools/call" => {
                let params = req.params.as_ref();
                let tool_name = params
                    .and_then(|p| p.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let args = params
                    .and_then(|p| p.get("arguments"))
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                let result = tools::call_tool(&self.prime, &self.recall, tool_name, &args).await;
                Some(Response::success(req.id.clone(), result))
            }

            "resources/list" => {
                let mut resources = vec![serde_json::json!({
                    "uri": "prime://cookbook",
                    "name": "prime_cookbook",
                    "description": "Usage patterns and best practices for Prime tools. Read this to learn the recommended workflows.",
                    "mimeType": "text/markdown"
                })];

                if self.auto_inject {
                    resources.push(serde_json::json!({
                        "uri": "prime://auto-context",
                        "name": "prime_auto_context",
                        "description": "Compressed knowledge index for system prompt injection. Updates automatically as memory grows.",
                        "mimeType": "text/markdown"
                    }));
                }

                Some(Response::success(
                    req.id.clone(),
                    serde_json::json!({ "resources": resources }),
                ))
            }

            "resources/read" => {
                let uri = req
                    .params
                    .as_ref()
                    .and_then(|p| p.get("uri"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                match uri {
                    "prime://cookbook" => Some(Response::success(
                        req.id.clone(),
                        serde_json::json!({
                            "contents": [{
                                "uri": "prime://cookbook",
                                "mimeType": "text/markdown",
                                "text": COOKBOOK
                            }]
                        }),
                    )),

                    "prime://auto-context" if self.auto_inject => {
                        let index = self.recall.index().await;
                        let markdown = if index.token_count > self.auto_inject_max_tokens {
                            let target_words = self.auto_inject_max_tokens * 10 / 13;
                            let truncated: String = index
                                .markdown
                                .split_whitespace()
                                .take(target_words)
                                .collect::<Vec<_>>()
                                .join(" ");
                            format!(
                                "{truncated}\n...(truncated to {} tokens)",
                                self.auto_inject_max_tokens
                            )
                        } else {
                            index.markdown
                        };

                        Some(Response::success(
                            req.id.clone(),
                            serde_json::json!({
                                "contents": [{
                                    "uri": "prime://auto-context",
                                    "mimeType": "text/markdown",
                                    "text": markdown
                                }]
                            }),
                        ))
                    }

                    _ => Some(Response::error(
                        req.id.clone(),
                        -32602,
                        format!("Unknown resource: {uri}"),
                    )),
                }
            }

            _ => Some(Response::error(
                req.id.clone(),
                -32601,
                format!("Method not found: {}", req.method),
            )),
        }
    }
}

/// Read a single MCP message from the reader.
///
/// Supports two modes:
/// 1. **Content-Length framing** (MCP spec): headers ending with blank line, then exact body bytes
/// 2. **Line-delimited fallback**: if first line starts with `{`, treat as line-delimited JSON
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

/// Write a response with Content-Length header framing.
fn write_response(writer: &mut impl Write, resp: &Response) -> Result<()> {
    let json = serde_json::to_string(resp)?;
    tracing::debug!("send: {json}");
    write!(writer, "Content-Length: {}\r\n\r\n{}", json.len(), json)?;
    writer.flush()?;
    Ok(())
}
