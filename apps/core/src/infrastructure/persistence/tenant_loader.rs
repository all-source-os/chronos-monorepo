//! Per-tenant lazy-load bookkeeping (Step 2 of the sustainable data
//! strategy).
//!
//! Boot used to read every Parquet file into a single shared
//! in-memory pile. Once the dataset crossed the available memory
//! cap, Core OOM'd during recovery (issue #160). Step 2 replaces
//! the boot-time full-load with on-demand per-tenant hydration:
//! the first query for tenant `T` triggers `load_events_for_tenant(T)`,
//! and subsequent queries hit the now-warm cache.
//!
//! This module owns the bookkeeping — which tenants are loaded, and a
//! per-tenant lock so two concurrent first-queries for the same
//! tenant don't double-load. The actual disk read lives in
//! `ParquetStorage::load_events_for_tenant`; the side-effect of
//! splicing loaded events into the in-memory index/projection
//! structures lives in `EventStore::ensure_tenant_loaded`.
//!
//! Step 3 will add a memory-budgeted cache that evicts cold tenants;
//! at that point the `loaded` set becomes "currently in memory"
//! rather than "ever loaded since boot", and a `mark_unloaded`
//! method shows up.

use dashmap::DashMap;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;

/// Default ceiling on how long a query may wait for an in-flight
/// load of the same tenant before giving up. The 100k-event load
/// budget for cold tenants is single-digit seconds (per the Step 2
/// acceptance criteria); 30s is comfortable headroom for that, well
/// short of a request-timeout indistinguishable from a hang.
pub const DEFAULT_LOAD_TIMEOUT: Duration = Duration::from_secs(30);

/// Tracks which tenants have been hydrated into memory, serializes
/// concurrent first-loads of the same tenant, and accounts for the
/// approximate bytes each loaded tenant occupies (Step 3 cache
/// budget input).
///
/// `loaded` is the source of truth for "is this tenant warm?". It's
/// insert-only until Step 3 #2's `mark_unloaded` (eviction) lands.
///
/// `locks` holds one Mutex per tenant ever queried. Concurrent
/// first-queries get the same Mutex; the second waiter re-checks
/// `loaded` after acquiring it (the standard double-checked-lock
/// pattern, sound here because `loaded`'s DashMap insert
/// happens-before the lock release).
///
/// `bytes` accumulates per-tenant resident-byte estimates as
/// events get spliced in via `append_loaded_event`. It's the input
/// the budget-enforcement step (Step 3 #3) keys off — the LRU
/// eviction policy compares the sum of these counters against the
/// configured byte budget.
pub struct TenantLoader {
    loaded: DashMap<String, ()>,
    locks: DashMap<String, Arc<Mutex<()>>>,
    bytes: DashMap<String, u64>,
    load_timeout: Duration,
}

impl TenantLoader {
    pub fn new() -> Self {
        Self::with_timeout(DEFAULT_LOAD_TIMEOUT)
    }

    pub fn with_timeout(load_timeout: Duration) -> Self {
        Self {
            loaded: DashMap::new(),
            locks: DashMap::new(),
            bytes: DashMap::new(),
            load_timeout,
        }
    }

    /// Fast path probe — true if a previous call to `mark_loaded`
    /// has succeeded for this tenant.
    pub fn is_loaded(&self, tenant_id: &str) -> bool {
        self.loaded.contains_key(tenant_id)
    }

    /// Record that this tenant has been hydrated. Idempotent.
    pub fn mark_loaded(&self, tenant_id: &str) {
        self.loaded.insert(tenant_id.to_string(), ());
    }

    /// Add `n` bytes to the resident-size estimate for `tenant_id`.
    /// Called once per event spliced into memory for that tenant.
    /// The total is what the budget check compares against.
    pub fn add_bytes(&self, tenant_id: &str, n: u64) {
        *self.bytes.entry(tenant_id.to_string()).or_insert(0) += n;
    }

    /// Resident-byte estimate for a single tenant. Returns 0 for
    /// tenants that have never been loaded (or that were evicted —
    /// once eviction lands the counter resets to 0).
    pub fn bytes_for(&self, tenant_id: &str) -> u64 {
        self.bytes.get(tenant_id).map_or(0, |v| *v)
    }

    /// Sum of resident-byte estimates across every loaded tenant —
    /// the input the budget check compares against. O(loaded
    /// tenants), expected to be small.
    pub fn total_bytes(&self) -> u64 {
        self.bytes.iter().map(|kv| *kv.value()).sum()
    }

    /// Snapshot of `(tenant_id, bytes)` pairs for every tenant
    /// that has any resident bytes. Used by the eviction policy
    /// to pick a victim and by metrics endpoints.
    pub fn bytes_per_tenant(&self) -> Vec<(String, u64)> {
        self.bytes
            .iter()
            .map(|kv| (kv.key().clone(), *kv.value()))
            .collect()
    }

    /// Get-or-insert the per-tenant Mutex used for singleflight
    /// loading. The first caller for a given tenant creates the
    /// Mutex; later callers see the same instance and serialize on
    /// it. Returns an `Arc` so the caller can hold the lock guard
    /// without keeping a borrow into the DashMap.
    pub fn lock_for(&self, tenant_id: &str) -> Arc<Mutex<()>> {
        self.locks
            .entry(tenant_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    pub fn load_timeout(&self) -> Duration {
        self.load_timeout
    }

    /// Number of tenants currently marked loaded. Diagnostic only.
    #[cfg(test)]
    pub fn loaded_count(&self) -> usize {
        self.loaded.len()
    }
}

impl Default for TenantLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    #[test]
    fn test_is_loaded_false_until_marked() {
        let loader = TenantLoader::new();
        assert!(!loader.is_loaded("alice"));
        loader.mark_loaded("alice");
        assert!(loader.is_loaded("alice"));
        // Other tenants stay cold — independent state per tenant.
        assert!(!loader.is_loaded("bob"));
    }

    #[test]
    fn test_mark_loaded_is_idempotent() {
        let loader = TenantLoader::new();
        loader.mark_loaded("alice");
        loader.mark_loaded("alice");
        assert_eq!(loader.loaded_count(), 1);
    }

    #[test]
    fn test_lock_for_returns_same_mutex_per_tenant() {
        // Two callers for the same tenant must get the same lock —
        // otherwise they don't serialize and the singleflight guarantee
        // is broken.
        let loader = TenantLoader::new();
        let lock_a1 = loader.lock_for("alice");
        let lock_a2 = loader.lock_for("alice");
        let lock_b = loader.lock_for("bob");

        assert!(Arc::ptr_eq(&lock_a1, &lock_a2));
        assert!(!Arc::ptr_eq(&lock_a1, &lock_b));
    }

    #[test]
    fn test_singleflight_blocks_second_caller_until_first_releases() {
        // Mimics the lazy-load path: caller 1 holds the Mutex while
        // "loading", caller 2 must wait until caller 1 releases.
        // We assert ordering by recording each thread's progress in a
        // counter.
        let loader = Arc::new(TenantLoader::new());
        let progress = Arc::new(AtomicUsize::new(0));

        let l1 = loader.clone();
        let p1 = progress.clone();
        let t1 = thread::spawn(move || {
            let lock = l1.lock_for("alice");
            let _g = lock.lock();
            // Caller 1 is now "loading" — record that we have the lock.
            p1.store(1, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(80));
            p1.store(2, Ordering::SeqCst);
            l1.mark_loaded("alice");
        });

        // Make sure thread 1 has acquired the lock before thread 2
        // starts. Avoids a flaky race where t2 acquires first.
        while progress.load(Ordering::SeqCst) < 1 {
            thread::sleep(Duration::from_millis(5));
        }

        let l2 = loader.clone();
        let p2 = progress.clone();
        let t2 = thread::spawn(move || {
            let lock = l2.lock_for("alice");
            let _g = lock.lock();
            // When we get here, t1 must already have advanced to 2.
            assert_eq!(
                p2.load(Ordering::SeqCst),
                2,
                "second caller acquired lock before first finished — singleflight broken"
            );
            // And the tenant must now be marked loaded.
            assert!(l2.is_loaded("alice"));
        });

        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn test_bytes_default_to_zero() {
        let loader = TenantLoader::new();
        assert_eq!(loader.bytes_for("alice"), 0);
        assert_eq!(loader.total_bytes(), 0);
        assert!(loader.bytes_per_tenant().is_empty());
    }

    #[test]
    fn test_add_bytes_accumulates_per_tenant() {
        let loader = TenantLoader::new();
        loader.add_bytes("alice", 100);
        loader.add_bytes("alice", 50);
        loader.add_bytes("bob", 200);
        assert_eq!(loader.bytes_for("alice"), 150);
        assert_eq!(loader.bytes_for("bob"), 200);
        assert_eq!(loader.total_bytes(), 350);

        let mut snapshot = loader.bytes_per_tenant();
        snapshot.sort();
        assert_eq!(snapshot, vec![("alice".to_string(), 150), ("bob".to_string(), 200)]);
    }

    #[test]
    fn test_lock_for_distinct_tenants_does_not_serialize() {
        // Two tenants must get distinct locks — alice loading must
        // not block bob's queries.
        let loader = Arc::new(TenantLoader::new());

        let l1 = loader.clone();
        let alice_lock = l1.lock_for("alice");
        let _alice_held = alice_lock.lock();

        // bob's lock should be acquirable immediately even while
        // alice's is held. try_lock returns Some(_) on success.
        let bob_lock = loader.lock_for("bob");
        let bob_held = bob_lock.try_lock();
        assert!(
            bob_held.is_some(),
            "bob's lock should not be blocked by alice's"
        );
    }
}
