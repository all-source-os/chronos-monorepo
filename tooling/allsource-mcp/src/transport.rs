//! MCP stdio transport — reads JSON-RPC from stdin, writes to stdout.

use allsource_core::embedded::EmbeddedCore;
use anyhow::Result;
use std::io::{BufRead, Write};

use crate::{
    protocol::{self, Request, Response},
    tools,
};

pub struct StdioTransport {
    core: EmbeddedCore,
}

impl StdioTransport {
    pub fn new(core: EmbeddedCore) -> Self {
        Self { core }
    }

    pub async fn run(&mut self) -> Result<()> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();

        for line in stdin.lock().lines() {
            let Ok(line) = line else {
                break; // EOF
            };

            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            tracing::debug!("recv: {line}");

            let request: Request = match serde_json::from_str(&line) {
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
        self.core.shutdown().await?;
        Ok(())
    }

    async fn handle_request(&self, req: &Request) -> Option<Response> {
        match req.method.as_str() {
            "initialize" => Some(Response::success(req.id.clone(), protocol::server_info())),

            // Notification — no response
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

                let result = tools::execute_tool(&self.core, tool_name, &args).await;
                Some(Response::success(req.id.clone(), result))
            }

            // Ignore other notifications silently
            method if method.starts_with("notifications/") => None,

            _ => Some(Response::error(
                req.id.clone(),
                -32601,
                format!("Method not found: {}", req.method),
            )),
        }
    }
}

fn write_response(stdout: &mut impl Write, response: &Response) -> Result<()> {
    let json = serde_json::to_string(response)?;
    tracing::debug!("send: {json}");
    writeln!(stdout, "{json}")?;
    stdout.flush()?;
    Ok(())
}
