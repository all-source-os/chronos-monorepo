use crate::domain::entities::Event;
use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// WebSocket manager for real-time event streaming (v0.2 feature)
pub struct WebSocketManager {
    /// Broadcast channel for sending events to all connected clients
    event_tx: broadcast::Sender<Arc<Event>>,

    /// Connected clients by ID - using DashMap for lock-free concurrent access
    clients: Arc<DashMap<Uuid, ClientInfo>>,
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
        let (event_tx, _) = broadcast::channel(1000);

        Self {
            event_tx,
            clients: Arc::new(DashMap::new()),
        }
    }

    /// Broadcast an event to all connected WebSocket clients
    pub fn broadcast_event(&self, event: Arc<Event>) {
        // Send to broadcast channel (non-blocking)
        let _ = self.event_tx.send(event);
    }

    /// Handle a new WebSocket connection
    pub async fn handle_socket(&self, socket: WebSocket) {
        let client_id = Uuid::new_v4();
        tracing::info!("🔌 WebSocket client connected: {}", client_id);

        // Subscribe to broadcast channel
        let mut event_rx = self.event_tx.subscribe();

        // Split socket into sender and receiver
        let (mut sender, mut receiver) = socket.split();

        // Register client
        self.clients.insert(
            client_id,
            ClientInfo {
                id: client_id,
                filters: EventFilters::default(),
            },
        );

        // Spawn task to send events to this client
        let clients = Arc::clone(&self.clients);
        let send_task = tokio::spawn(async move {
            while let Ok(event) = event_rx.recv().await {
                // Get client filters
                let filters = clients
                    .get(&client_id)
                    .map(|entry| entry.value().filters.clone())
                    .unwrap_or_default();

                // Apply filters
                if let Some(ref entity_id) = filters.entity_id
                    && event.entity_id_str() != entity_id
                {
                    continue;
                }

                if let Some(ref event_type) = filters.event_type
                    && event.event_type_str() != event_type
                {
                    continue;
                }

                // Serialize event to JSON
                match serde_json::to_string(&*event) {
                    Ok(json) => {
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            tracing::warn!("Failed to send event to client {}", client_id);
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to serialize event: {}", e);
                    }
                }
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
}
