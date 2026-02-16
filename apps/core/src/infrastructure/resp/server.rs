//! RESP3 TCP server.
//!
//! Accepts Redis-protocol connections and dispatches commands to the EventStore.
//! Runs alongside the HTTP API server on a separate port (default 6380).

use crate::{domain::entities::Event, store::EventStore};
use std::sync::Arc;
use tokio::{
    io::BufReader,
    net::{TcpListener, TcpStream},
};

use super::{
    commands,
    protocol::{self, RespValue},
};

/// RESP3 server that bridges Redis wire protocol to EventStore operations.
pub struct RespServer {
    store: Arc<EventStore>,
}

impl RespServer {
    pub fn new(store: Arc<EventStore>) -> Self {
        Self { store }
    }

    /// Start accepting connections on the given port.
    ///
    /// This function runs forever (until the process shuts down).
    pub async fn serve(self: Arc<Self>, port: u16) -> anyhow::Result<()> {
        let addr = format!("0.0.0.0:{port}");
        let listener = TcpListener::bind(&addr).await?;

        tracing::info!("RESP3 server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    tracing::debug!("RESP3 client connected from {}", peer_addr);
                    let server = Arc::clone(&self);
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream).await {
                            tracing::debug!("RESP3 client {} disconnected: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("RESP3 accept error: {}", e);
                }
            }
        }
    }

    async fn handle_connection(&self, stream: TcpStream) -> anyhow::Result<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        loop {
            let value = match protocol::parse_value(&mut reader).await {
                Ok(Some(v)) => v,
                Ok(None) => return Ok(()), // EOF
                Err(e) => {
                    let _ = protocol::write_value(
                        &mut writer,
                        &RespValue::err(format!("protocol error: {e}")),
                    )
                    .await;
                    return Err(e.into());
                }
            };

            // Commands come as arrays of bulk strings
            let args = match value {
                RespValue::Array(items) => items,
                other => {
                    // Inline command support: simple string treated as single-arg command
                    if let Some(s) = other.as_str() {
                        s.split_whitespace().map(RespValue::bulk_string).collect()
                    } else {
                        protocol::write_value(
                            &mut writer,
                            &RespValue::err("expected array or inline command"),
                        )
                        .await?;
                        continue;
                    }
                }
            };

            // Check for QUIT
            if let Some(cmd) = args.first().and_then(|v| v.as_str())
                && cmd.eq_ignore_ascii_case("QUIT")
            {
                protocol::write_value(&mut writer, &RespValue::ok()).await?;
                return Ok(());
            }

            let (response, subscription) = commands::execute(&args, &self.store);

            // Write the response
            protocol::write_value(&mut writer, &response).await?;

            // If we got a subscription, enter pub/sub mode
            if let Some(mut rx) = subscription {
                self.run_subscription(&mut writer, &mut rx).await?;
                return Ok(()); // subscription mode exits on error/disconnect
            }
        }
    }

    /// Run in subscription mode: forward broadcast events to the client
    /// as Redis pub/sub messages until the connection drops.
    async fn run_subscription(
        &self,
        writer: &mut (impl tokio::io::AsyncWrite + Unpin),
        rx: &mut tokio::sync::broadcast::Receiver<Arc<Event>>,
    ) -> anyhow::Result<()> {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    // Format as Redis pub/sub message:
                    // *3\r\n$7\r\nmessage\r\n$<channel_len>\r\n<channel>\r\n$<payload_len>\r\n<payload>\r\n
                    let channel = format!("events:{}", event.event_type_str());
                    let payload =
                        serde_json::to_string(event.as_ref()).unwrap_or_else(|_| "{}".to_string());

                    let msg = RespValue::Array(vec![
                        RespValue::bulk_string("message"),
                        RespValue::bulk_string(&channel),
                        RespValue::bulk_string(&payload),
                    ]);

                    if let Err(e) = protocol::write_value(writer, &msg).await {
                        tracing::debug!("RESP3 subscription write error: {}", e);
                        return Ok(());
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("RESP3 subscriber lagged by {} messages", n);
                    // Continue receiving — we just lost some messages
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    return Ok(());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };

    /// Helper: start a server on a random port, return the port.
    async fn start_test_server() -> (u16, Arc<EventStore>) {
        let store = Arc::new(EventStore::new());
        let server = Arc::new(RespServer::new(Arc::clone(&store)));

        // Bind to port 0 to get a random available port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server_clone = Arc::clone(&server);
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let s = Arc::clone(&server_clone);
                tokio::spawn(async move {
                    let _ = s.handle_connection(stream).await;
                });
            }
        });

        (port, store)
    }

    async fn send_command(stream: &mut TcpStream, parts: &[&str]) -> String {
        // Build RESP array
        let cmd = RespValue::Array(parts.iter().map(|s| RespValue::bulk_string(s)).collect());
        stream.write_all(&cmd.encode()).await.unwrap();
        stream.flush().await.unwrap();

        // Read response (simple: read available bytes)
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let mut buf = vec![0u8; 4096];
        let n = tokio::time::timeout(std::time::Duration::from_millis(200), stream.read(&mut buf))
            .await
            .unwrap_or(Ok(0))
            .unwrap_or(0);
        String::from_utf8_lossy(&buf[..n]).to_string()
    }

    #[tokio::test]
    async fn test_server_ping() {
        let (port, _store) = start_test_server().await;
        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();

        let resp = send_command(&mut stream, &["PING"]).await;
        assert!(resp.contains("PONG"), "got: {resp}");
    }

    #[tokio::test]
    async fn test_server_xadd_xrange() {
        let (port, _store) = start_test_server().await;
        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();

        // XADD
        let resp = send_command(
            &mut stream,
            &[
                "XADD",
                "default",
                "*",
                "event_type",
                "user.created",
                "entity_id",
                "user-1",
            ],
        )
        .await;
        assert!(
            resp.contains("-0"),
            "stream ID should end in -0, got: {resp}"
        );

        // XRANGE
        let resp = send_command(&mut stream, &["XRANGE", "default", "-", "+"]).await;
        assert!(resp.contains("user.created"), "got: {resp}");
    }

    #[tokio::test]
    async fn test_server_quit() {
        let (port, _store) = start_test_server().await;
        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();

        let resp = send_command(&mut stream, &["QUIT"]).await;
        assert!(resp.contains("OK"), "got: {resp}");
    }
}
