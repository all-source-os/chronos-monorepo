// Web layer - HTTP handlers, WebSocket, and API routes
// Clean Architecture: Infrastructure Layer

pub mod api;
pub mod api_v1;
pub mod audit_api;
pub mod auth_api;
pub mod config_api;
pub mod demo_api;
pub mod handlers;
pub mod tenant_api;
pub mod websocket;

// Public API re-exports
pub use api::serve;
pub use websocket::{EventFilters, WebSocketManager, WebSocketStats};

// Handler state re-exports for convenience
pub use handlers::{
    ArticleHandlerState, CreatorHandlerState, ForkHandlerState, PaymentHandlerState,
};
