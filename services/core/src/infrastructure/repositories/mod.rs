pub mod in_memory_event_stream_repository;
pub mod in_memory_audit_repository;
pub mod in_memory_tenant_repository;

#[cfg(feature = "postgres")]
pub mod postgres_event_stream_repository;

#[cfg(feature = "postgres")]
pub mod postgres_audit_repository;

#[cfg(feature = "postgres")]
pub mod postgres_tenant_repository;

#[cfg(feature = "rocksdb-storage")]
pub mod rocksdb_event_stream_repository;

pub use in_memory_event_stream_repository::InMemoryEventStreamRepository;
pub use in_memory_audit_repository::InMemoryAuditRepository;
pub use in_memory_tenant_repository::InMemoryTenantRepository;

#[cfg(feature = "postgres")]
pub use postgres_event_stream_repository::PostgresEventStreamRepository;

#[cfg(feature = "postgres")]
pub use postgres_audit_repository::PostgresAuditRepository;

#[cfg(feature = "postgres")]
pub use postgres_tenant_repository::PostgresTenantRepository;

#[cfg(feature = "rocksdb-storage")]
pub use rocksdb_event_stream_repository::RocksDBEventStreamRepository;
