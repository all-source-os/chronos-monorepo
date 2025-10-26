pub mod in_memory_event_stream_repository;

#[cfg(feature = "postgres")]
pub mod postgres_event_stream_repository;

#[cfg(feature = "rocksdb-storage")]
pub mod rocksdb_event_stream_repository;

pub use in_memory_event_stream_repository::InMemoryEventStreamRepository;

#[cfg(feature = "postgres")]
pub use postgres_event_stream_repository::PostgresEventStreamRepository;

#[cfg(feature = "rocksdb-storage")]
pub use rocksdb_event_stream_repository::RocksDBEventStreamRepository;
