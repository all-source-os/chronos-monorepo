// Web handlers module - HTTP handlers that delegate to application services
// Clean Architecture: Infrastructure Layer -> Application Layer

pub mod article_handlers;
pub mod creator_handlers;
pub mod fork_handlers;
pub mod payment_handlers;

pub use article_handlers::*;
pub use creator_handlers::*;
pub use fork_handlers::*;
pub use payment_handlers::*;
