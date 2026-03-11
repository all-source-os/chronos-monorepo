/// Hybrid Logical Clock (HLC) for geo-replication.
///
/// Combines physical wall-clock time with a logical counter to provide
/// causally consistent timestamps across distributed nodes. Based on the
/// HLC algorithm from Kulkarni et al. (2014).
///
/// Properties:
/// - Monotonically increasing on each node
/// - Causally consistent across nodes (send/receive semantics)
/// - Close to physical time (bounded drift)
/// - No coordination required between nodes
///
/// # Timestamp Format
///
/// An HLC timestamp is a 128-bit value: `(physical_ms: u64, logical: u32, node_id: u32)`
/// - `physical_ms`: milliseconds since Unix epoch
/// - `logical`: counter that breaks ties when physical time hasn't advanced
/// - `node_id`: originating node (for total ordering)
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    time::{SystemTime, UNIX_EPOCH},
};

/// A hybrid logical clock timestamp.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct HlcTimestamp {
    /// Physical time in milliseconds since Unix epoch.
    pub physical_ms: u64,
    /// Logical counter (monotonically increasing within the same physical ms).
    pub logical: u32,
    /// Node ID that generated this timestamp.
    pub node_id: u32,
}

impl HlcTimestamp {
    /// Create a new HLC timestamp.
    pub fn new(physical_ms: u64, logical: u32, node_id: u32) -> Self {
        Self {
            physical_ms,
            logical,
            node_id,
        }
    }

    /// Zero timestamp (used as initial value).
    pub fn zero() -> Self {
        Self {
            physical_ms: 0,
            logical: 0,
            node_id: 0,
        }
    }

    /// Encode as a u128 for compact storage and comparison.
    /// Format: `[physical_ms:64][logical:32][node_id:32]`
    pub fn to_u128(&self) -> u128 {
        (u128::from(self.physical_ms) << 64)
            | (u128::from(self.logical) << 32)
            | u128::from(self.node_id)
    }

    /// Decode from a u128.
    pub fn from_u128(val: u128) -> Self {
        Self {
            physical_ms: (val >> 64) as u64,
            logical: ((val >> 32) & 0xFFFF_FFFF) as u32,
            node_id: (val & 0xFFFF_FFFF) as u32,
        }
    }
}

impl Ord for HlcTimestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.physical_ms
            .cmp(&other.physical_ms)
            .then(self.logical.cmp(&other.logical))
            .then(self.node_id.cmp(&other.node_id))
    }
}

impl PartialOrd for HlcTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for HlcTimestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.physical_ms, self.logical, self.node_id)
    }
}

/// Hybrid Logical Clock state for a single node.
///
/// Thread-safe via a Mutex protecting the (physical_ms, logical) pair.
/// This ensures the pair is updated atomically, preventing duplicate
/// timestamps under high contention.
pub struct HybridLogicalClock {
    /// This node's ID.
    node_id: u32,
    /// Packed state: (physical_ms, logical). Protected by Mutex for atomicity.
    state: std::sync::Mutex<(u64, u32)>,
    /// Maximum allowed drift from wall clock (ms). Events with timestamps
    /// further ahead than this are rejected.
    max_drift_ms: u64,
}

impl HybridLogicalClock {
    /// Create a new HLC for the given node.
    pub fn new(node_id: u32) -> Self {
        Self {
            node_id,
            state: std::sync::Mutex::new((0, 0)),
            max_drift_ms: 60_000, // 1 minute default
        }
    }

    /// Create with a custom max drift tolerance.
    pub fn with_max_drift(node_id: u32, max_drift_ms: u64) -> Self {
        Self {
            node_id,
            state: std::sync::Mutex::new((0, 0)),
            max_drift_ms,
        }
    }

    /// Get the node ID.
    pub fn node_id(&self) -> u32 {
        self.node_id
    }

    /// Get the current wall clock time in milliseconds.
    fn wall_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Generate a new timestamp for a local event.
    ///
    /// Algorithm (from HLC paper):
    /// 1. pt = max(last_physical, wall_clock)
    /// 2. if pt == last_physical: logical++, else logical = 0
    /// 3. last_physical = pt
    pub fn now(&self) -> HlcTimestamp {
        let wall = Self::wall_ms();
        let mut guard = self.state.lock().unwrap();
        let (last_pt, last_l) = *guard;

        let (new_pt, new_l) = if wall > last_pt {
            (wall, 0)
        } else {
            (last_pt, last_l + 1)
        };

        *guard = (new_pt, new_l);
        HlcTimestamp::new(new_pt, new_l, self.node_id)
    }

    /// Update the clock on receiving a remote message/event.
    ///
    /// Algorithm:
    /// 1. pt = max(last_physical, remote_physical, wall_clock)
    /// 2. Compute logical based on which components contributed to pt
    /// 3. Validate drift bounds
    ///
    /// Returns `Err` if the remote timestamp is too far ahead (drift violation).
    pub fn receive(&self, remote: &HlcTimestamp) -> Result<HlcTimestamp, HlcDriftError> {
        let wall = Self::wall_ms();

        // Check drift: remote timestamp shouldn't be too far ahead of our wall clock
        if remote.physical_ms > wall + self.max_drift_ms {
            return Err(HlcDriftError {
                remote_ms: remote.physical_ms,
                local_wall_ms: wall,
                max_drift_ms: self.max_drift_ms,
            });
        }

        let mut guard = self.state.lock().unwrap();
        let (last_pt, last_l) = *guard;

        let new_pt = wall.max(last_pt).max(remote.physical_ms);

        let new_l = if new_pt == last_pt && new_pt == remote.physical_ms {
            // All three tied — advance logical past both
            last_l.max(remote.logical) + 1
        } else if new_pt == last_pt {
            // Wall advanced past remote, but matched our last
            last_l + 1
        } else if new_pt == remote.physical_ms {
            // Remote is the most recent physical time
            remote.logical + 1
        } else {
            // Wall clock is strictly the greatest — reset logical
            0
        };

        *guard = (new_pt, new_l);
        Ok(HlcTimestamp::new(new_pt, new_l, self.node_id))
    }

    /// Get the current HLC state without advancing it.
    pub fn current(&self) -> HlcTimestamp {
        let guard = self.state.lock().unwrap();
        HlcTimestamp::new(guard.0, guard.1, self.node_id)
    }
}

/// Error when a remote HLC timestamp exceeds the max allowed drift.
#[derive(Debug, Clone)]
pub struct HlcDriftError {
    pub remote_ms: u64,
    pub local_wall_ms: u64,
    pub max_drift_ms: u64,
}

impl std::fmt::Display for HlcDriftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HLC drift violation: remote={}, local_wall={}, max_drift={}",
            self.remote_ms, self.local_wall_ms, self.max_drift_ms,
        )
    }
}

impl std::error::Error for HlcDriftError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hlc_now_monotonic() {
        let hlc = HybridLogicalClock::new(1);
        let t1 = hlc.now();
        let t2 = hlc.now();
        let t3 = hlc.now();
        assert!(t1 < t2);
        assert!(t2 < t3);
    }

    #[test]
    fn test_hlc_now_uses_wall_clock() {
        let hlc = HybridLogicalClock::new(1);
        let before = HybridLogicalClock::wall_ms();
        let ts = hlc.now();
        let after = HybridLogicalClock::wall_ms();
        assert!(ts.physical_ms >= before);
        assert!(ts.physical_ms <= after + 1); // +1 for timing tolerance
    }

    #[test]
    fn test_hlc_receive_advances_past_remote() {
        let hlc = HybridLogicalClock::new(1);
        let _ = hlc.now(); // initialize local state

        // Simulate a remote timestamp far in the future (but within drift)
        let remote = HlcTimestamp::new(HybridLogicalClock::wall_ms() + 100, 5, 2);
        let ts = hlc.receive(&remote).unwrap();
        assert!(ts > remote);
    }

    #[test]
    fn test_hlc_receive_drift_rejected() {
        let hlc = HybridLogicalClock::with_max_drift(1, 1000); // 1 second max drift
        let remote = HlcTimestamp::new(HybridLogicalClock::wall_ms() + 5000, 0, 2);
        assert!(hlc.receive(&remote).is_err());
    }

    #[test]
    fn test_hlc_receive_same_physical_time() {
        let hlc = HybridLogicalClock::new(1);
        let wall = HybridLogicalClock::wall_ms();

        // Set local state to current wall time with logical=5
        let remote1 = HlcTimestamp::new(wall, 5, 2);
        let ts1 = hlc.receive(&remote1).unwrap();
        assert!(ts1.logical > 5);

        // Another remote at same physical time but higher logical
        let remote2 = HlcTimestamp::new(wall, 10, 3);
        let ts2 = hlc.receive(&remote2).unwrap();
        assert!(ts2 > ts1);
    }

    #[test]
    fn test_hlc_timestamp_ordering() {
        let a = HlcTimestamp::new(100, 0, 1);
        let b = HlcTimestamp::new(100, 1, 1);
        let c = HlcTimestamp::new(101, 0, 1);
        let d = HlcTimestamp::new(100, 0, 2);

        assert!(a < b); // same physical, higher logical
        assert!(b < c); // higher physical wins
        assert!(a < d); // same physical+logical, higher node_id
    }

    #[test]
    fn test_hlc_timestamp_u128_roundtrip() {
        let ts = HlcTimestamp::new(1_700_000_000_000, 42, 7);
        let encoded = ts.to_u128();
        let decoded = HlcTimestamp::from_u128(encoded);
        assert_eq!(ts, decoded);
    }

    #[test]
    fn test_hlc_timestamp_display() {
        let ts = HlcTimestamp::new(1000, 5, 3);
        assert_eq!(ts.to_string(), "1000:5:3");
    }

    #[test]
    fn test_hlc_zero() {
        let z = HlcTimestamp::zero();
        assert_eq!(z.physical_ms, 0);
        assert_eq!(z.logical, 0);
        assert_eq!(z.node_id, 0);
    }

    #[test]
    fn test_hlc_concurrent_access() {
        use std::sync::Arc;
        let hlc = Arc::new(HybridLogicalClock::new(1));
        let mut handles = vec![];

        for _ in 0..10 {
            let hlc = Arc::clone(&hlc);
            handles.push(std::thread::spawn(move || {
                let mut timestamps = Vec::new();
                for _ in 0..100 {
                    timestamps.push(hlc.now());
                }
                timestamps
            }));
        }

        let mut all: Vec<HlcTimestamp> = vec![];
        for h in handles {
            all.extend(h.join().unwrap());
        }

        // All timestamps should be unique
        let mut sorted = all.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            all.len(),
            "HLC must produce unique timestamps under contention"
        );
    }
}
