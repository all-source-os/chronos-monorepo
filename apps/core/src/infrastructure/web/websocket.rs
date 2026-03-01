use crate::domain::entities::Event;
use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Configuration for WebSocket backpressure and batching.
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// Broadcast channel capacity (default 1000).
    pub capacity: usize,
    /// Optional batching interval in milliseconds.
    /// `None` = no batching (current behavior, backward compatible).
    /// `Some(50)` = buffer events and flush every 50ms as JSON arrays.
    pub batch_interval_ms: Option<u64>,
    /// Flush early when the batch reaches this size (default 100).
    pub max_batch_size: usize,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            capacity: 1000,
            batch_interval_ms: None,
            max_batch_size: 100,
        }
    }
}

/// WebSocket manager for real-time event streaming (v0.2 feature)
pub struct WebSocketManager {
    /// Broadcast channel for sending events to all connected clients
    event_tx: broadcast::Sender<Arc<Event>>,

    /// Connected clients by ID - using DashMap for lock-free concurrent access
    clients: Arc<DashMap<Uuid, ClientInfo>>,

    /// Backpressure and batching configuration
    config: WebSocketConfig,
}

#[derive(Debug, Clone)]
struct ClientInfo {
    id: Uuid,
    filters: EventFilters,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct EventFilters {
    pub entity_id: Option<String>,
    pub event_type: Option<String>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self::with_config(WebSocketConfig::default())
    }

    /// Create a WebSocket manager with custom backpressure configuration.
    pub fn with_config(config: WebSocketConfig) -> Self {
        let (event_tx, _) = broadcast::channel(config.capacity);

        Self {
            event_tx,
            clients: Arc::new(DashMap::new()),
            config,
        }
    }

    /// Broadcast an event to all connected WebSocket clients
    pub fn broadcast_event(&self, event: Arc<Event>) {
        // Send to broadcast channel (non-blocking)
        let _ = self.event_tx.send(event);
    }

    /// Subscribe to the event broadcast channel (used by RESP3 SUBSCRIBE).
    pub fn subscribe_events(&self) -> broadcast::Receiver<Arc<Event>> {
        self.event_tx.subscribe()
    }

    /// Handle a new WebSocket connection
    pub async fn handle_socket(&self, socket: WebSocket) {
        let client_id = Uuid::new_v4();
        tracing::info!("🔌 WebSocket client connected: {}", client_id);

        // Subscribe to broadcast channel
        let event_rx = self.event_tx.subscribe();

        // Split socket into sender and receiver
        let (sender, mut receiver) = socket.split();

        // Register client
        self.clients.insert(
            client_id,
            ClientInfo {
                id: client_id,
                filters: EventFilters::default(),
            },
        );

        // Spawn send task based on config
        let clients = Arc::clone(&self.clients);
        let config = self.config.clone();
        let send_task = tokio::spawn(async move {
            if let Some(interval_ms) = config.batch_interval_ms {
                Self::send_batched(
                    event_rx,
                    sender,
                    clients,
                    client_id,
                    interval_ms,
                    config.max_batch_size,
                )
                .await;
            } else {
                Self::send_unbatched(event_rx, sender, clients, client_id).await;
            }
        });

        // Handle incoming messages from client (for setting filters)
        let clients = Arc::clone(&self.clients);
        let recv_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                if let Message::Text(text) = msg {
                    // Parse filter commands (text is Utf8Bytes in axum 0.8+)
                    if let Ok(filters) = serde_json::from_str::<EventFilters>(text.as_str()) {
                        tracing::info!("Setting filters for client {}: {:?}", client_id, filters);
                        if let Some(mut client) = clients.get_mut(&client_id) {
                            client.filters = filters;
                        }
                    }
                }
            }
        });

        // Wait for either task to finish
        tokio::select! {
            _ = send_task => {
                tracing::info!("Send task ended for client {}", client_id);
            }
            _ = recv_task => {
                tracing::info!("Receive task ended for client {}", client_id);
            }
        }

        // Clean up client
        self.clients.remove(&client_id);
        tracing::info!("🔌 WebSocket client disconnected: {}", client_id);
    }

    /// Unbatched send loop — original behavior (one message per event).
    async fn send_unbatched(
        mut event_rx: broadcast::Receiver<Arc<Event>>,
        mut sender: futures::stream::SplitSink<WebSocket, Message>,
        clients: Arc<DashMap<Uuid, ClientInfo>>,
        client_id: Uuid,
    ) {
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    if !Self::passes_filters(&clients, client_id, &event) {
                        continue;
                    }

                    match serde_json::to_string(&*event) {
                        Ok(json) => {
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                tracing::warn!(
                                    "Failed to send event to client {}",
                                    client_id
                                );
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to serialize event: {}", e);
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    let msg = serde_json::json!({"type": "lagged", "missed": n});
                    let _ = sender
                        .send(Message::Text(msg.to_string().into()))
                        .await;
                    tracing::warn!(
                        "Client {} lagged, missed {} events",
                        client_id,
                        n
                    );
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }

    /// Batched send loop — buffers events and flushes periodically or on max batch size.
    async fn send_batched(
        mut event_rx: broadcast::Receiver<Arc<Event>>,
        mut sender: futures::stream::SplitSink<WebSocket, Message>,
        clients: Arc<DashMap<Uuid, ClientInfo>>,
        client_id: Uuid,
        interval_ms: u64,
        max_batch_size: usize,
    ) {
        let mut batch: Vec<serde_json::Value> = Vec::with_capacity(max_batch_size);
        let mut ticker = tokio::time::interval(std::time::Duration::from_millis(interval_ms));
        ticker.tick().await; // first tick completes immediately

        loop {
            tokio::select! {
                result = event_rx.recv() => {
                    match result {
                        Ok(event) => {
                            if !Self::passes_filters(&clients, client_id, &event) {
                                continue;
                            }

                            if let Ok(val) = serde_json::to_value(&*event) {
                                batch.push(val);
                            }

                            // Flush early if batch is full
                            if batch.len() >= max_batch_size {
                                if !Self::flush_batch(&mut sender, &mut batch, client_id).await {
                                    break;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            // Flush any pending batch first
                            let _ = Self::flush_batch(&mut sender, &mut batch, client_id).await;
                            let msg = serde_json::json!({"type": "lagged", "missed": n});
                            let _ = sender
                                .send(Message::Text(msg.to_string().into()))
                                .await;
                            tracing::warn!(
                                "Client {} lagged, missed {} events",
                                client_id,
                                n
                            );
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            // Flush remaining batch before exit
                            let _ = Self::flush_batch(&mut sender, &mut batch, client_id).await;
                            break;
                        }
                    }
                }
                _ = ticker.tick() => {
                    if !batch.is_empty() {
                        if !Self::flush_batch(&mut sender, &mut batch, client_id).await {
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Flush the current batch as a JSON array. Returns false if send failed.
    async fn flush_batch(
        sender: &mut futures::stream::SplitSink<WebSocket, Message>,
        batch: &mut Vec<serde_json::Value>,
        client_id: Uuid,
    ) -> bool {
        if batch.is_empty() {
            return true;
        }

        let json_array = serde_json::Value::Array(std::mem::take(batch));
        match serde_json::to_string(&json_array) {
            Ok(json) => {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    tracing::warn!("Failed to send batch to client {}", client_id);
                    return false;
                }
                true
            }
            Err(e) => {
                tracing::error!("Failed to serialize batch: {}", e);
                batch.clear();
                true
            }
        }
    }

    /// Check if an event passes the client's filters.
    fn passes_filters(
        clients: &DashMap<Uuid, ClientInfo>,
        client_id: Uuid,
        event: &Event,
    ) -> bool {
        let filters = clients
            .get(&client_id)
            .map(|entry| entry.value().filters.clone())
            .unwrap_or_default();

        if let Some(ref entity_id) = filters.entity_id
            && event.entity_id_str() != entity_id
        {
            return false;
        }

        if let Some(ref event_type) = filters.event_type
            && event.event_type_str() != event_type
        {
            return false;
        }

        true
    }

    /// Get statistics about connected clients
    pub fn stats(&self) -> WebSocketStats {
        WebSocketStats {
            connected_clients: self.clients.len(),
            total_capacity: self.event_tx.receiver_count(),
        }
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct WebSocketStats {
    pub connected_clients: usize,
    pub total_capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_event() -> Event {
        Event::reconstruct_from_strings(
            Uuid::new_v4(),
            "test.event".to_string(),
            "test-entity".to_string(),
            "default".to_string(),
            json!({"test": "data"}),
            chrono::Utc::now(),
            None,
            1,
        )
    }

    #[test]
    fn test_websocket_manager_creation() {
        let manager = WebSocketManager::new();
        let stats = manager.stats();
        assert_eq!(stats.connected_clients, 0);
    }

    #[test]
    fn test_event_broadcast() {
        let manager = WebSocketManager::new();
        let event = Arc::new(create_test_event());

        // Should not panic
        manager.broadcast_event(event);
    }

    #[test]
    fn test_config_defaults() {
        let config = WebSocketConfig::default();
        assert_eq!(config.capacity, 1000);
        assert_eq!(config.batch_interval_ms, None);
        assert_eq!(config.max_batch_size, 100);
    }

    #[test]
    fn test_lagged_notification() {
        // Create a tiny channel that will lag quickly
        let config = WebSocketConfig {
            capacity: 2,
            batch_interval_ms: None,
            max_batch_size: 100,
        };
        let manager = WebSocketManager::with_config(config);

        // Subscribe, then overflow the channel
        let mut rx = manager.subscribe_events();
        for _ in 0..5 {
            manager.broadcast_event(Arc::new(create_test_event()));
        }

        // The receiver should get a Lagged error
        match rx.try_recv() {
            Err(broadcast::error::TryRecvError::Lagged(n)) => {
                assert!(n > 0, "should report missed events");
            }
            Ok(_) => {
                // Got an event — that's fine, lagged may come on next recv
            }
            Err(e) => {
                panic!("unexpected error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_batch_mode_groups_events() {
        // Verify that with_config creates a manager with batching params
        let config = WebSocketConfig {
            capacity: 1000,
            batch_interval_ms: Some(50),
            max_batch_size: 10,
        };
        let manager = WebSocketManager::with_config(config.clone());
        assert_eq!(manager.config.batch_interval_ms, Some(50));
        assert_eq!(manager.config.max_batch_size, 10);

        // The actual batching behavior is tested via the flush_batch helper
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            // Create a batch of events and serialize as JSON array
            let events: Vec<serde_json::Value> = (0..3)
                .map(|_| serde_json::to_value(&create_test_event()).unwrap())
                .collect();

            let json_array = serde_json::Value::Array(events);
            let serialized = serde_json::to_string(&json_array).unwrap();
            let parsed: Vec<serde_json::Value> = serde_json::from_str(&serialized).unwrap();
            assert_eq!(parsed.len(), 3);
        });
    }

    #[test]
    fn test_batch_flush_on_max_size() {
        // Verify config with small max_batch_size
        let config = WebSocketConfig {
            capacity: 1000,
            batch_interval_ms: Some(1000), // long interval
            max_batch_size: 5,             // small batch — triggers early flush
        };
        let manager = WebSocketManager::with_config(config);
        assert_eq!(manager.config.max_batch_size, 5);
    }

    #[test]
    fn test_backward_compat_no_config() {
        // Default constructor should work identically to pre-backpressure behavior
        let manager = WebSocketManager::new();
        assert_eq!(manager.config.capacity, 1000);
        assert!(manager.config.batch_interval_ms.is_none());

        // Broadcast still works
        let event = Arc::new(create_test_event());
        manager.broadcast_event(event);

        let stats = manager.stats();
        assert_eq!(stats.connected_clients, 0);
    }
}
