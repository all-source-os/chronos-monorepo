// Persistence infrastructure layer
// Contains storage implementations, WAL, snapshots, compaction, and indexing

pub mod arena_pool;
pub mod backup;
pub mod batch_processor;
pub mod compaction;
pub mod index;
pub mod lock_free;
pub mod performance;
#[cfg(test)]
mod performance_test;
pub mod simd_json;
pub mod snapshot;
pub mod storage;
pub mod storage_integrity;
pub mod wal;

// Re-exports for convenience
pub use arena_pool::{
    arena_stats, get_arena, get_arena_with_capacity, ArenaPoolStats, PooledArena, ScopedArena,
    SizedBufferPool,
};
pub use backup::*;
pub use batch_processor::{
    ArenaBatchBuffer, BatchProcessor, BatchProcessorConfig, BatchProcessorStats, BatchResult,
    RawEventData,
};
pub use compaction::{CompactionConfig, CompactionManager, CompactionResult, CompactionStrategy};
pub use index::{EventIndex, IndexEntry};
pub use lock_free::{
    LockFreeEventQueue, LockFreeMetrics, MetricsSnapshot, ShardedEventQueue, ShardedQueueStats,
};
pub use performance::{BatchWriter, MemoryPool, PerformanceMetrics};
pub use simd_json::{BatchEventParser, SimdJsonError, SimdJsonParser, SimdJsonStats, ZeroCopyJson};
pub use snapshot::{
    CreateSnapshotRequest, CreateSnapshotResponse, ListSnapshotsRequest, ListSnapshotsResponse,
    Snapshot, SnapshotConfig, SnapshotInfo, SnapshotManager, SnapshotType,
};
pub use storage::ParquetStorage;
pub use storage_integrity::{IntegrityCheckResult, StorageIntegrity};
pub use wal::{WALConfig, WriteAheadLog};
