/// CRDT-based Conflict Resolution for Geo-Replicated Events.
///
/// When events are ingested concurrently at different regions, conflicts are
/// resolved deterministically without coordination using CRDTs (Conflict-free
/// Replicated Data Types).
///
/// # Strategy
///
/// Events are immutable append-only facts — they use a **G-Set** (grow-only set)
/// CRDT by nature. The conflict resolution challenge is in **ordering**: when two
/// regions each write events at the same logical time, we need a deterministic
/// total order for consistency.
///
/// ## Resolution Rules (in priority order)
///
/// 1. **HLC timestamp**: Higher physical time wins
/// 2. **Logical counter**: If physical times are equal, higher logical counter wins
/// 3. **Node ID**: If both are equal, lower node ID wins (deterministic tiebreak)
/// 4. **Event ID (UUID)**: Final tiebreak on lexicographic UUID comparison
///
/// This produces a total order that all regions converge to identically,
/// without any coordination.
///
/// # Version Vectors
///
/// Each region maintains a version vector tracking the latest HLC timestamp
/// seen from every other region. During replication:
/// - Incoming events are accepted if their HLC > the vector entry for their origin
/// - The vector is updated on receipt
/// - This prevents duplicate delivery and enables efficient delta sync
use super::hlc::HlcTimestamp;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Version vector tracking causal progress per region.
///
/// Maps region_id → latest HLC timestamp seen from that region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionVector {
    /// Region ID → latest HLC timestamp from that region.
    entries: BTreeMap<String, HlcTimestamp>,
}

impl VersionVector {
    /// Create an empty version vector.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Update the vector entry for a region if the given timestamp is newer.
    /// Returns true if the vector was actually updated (i.e., the event is new).
    pub fn advance(&mut self, region_id: &str, ts: HlcTimestamp) -> bool {
        match self.entries.get(region_id) {
            Some(existing) if ts <= *existing => false,
            _ => {
                self.entries.insert(region_id.to_string(), ts);
                true
            }
        }
    }

    /// Check if a timestamp from a region is new (not yet seen).
    pub fn is_new(&self, region_id: &str, ts: &HlcTimestamp) -> bool {
        match self.entries.get(region_id) {
            Some(existing) => ts > existing,
            None => true,
        }
    }

    /// Get the latest timestamp for a region.
    pub fn get(&self, region_id: &str) -> Option<&HlcTimestamp> {
        self.entries.get(region_id)
    }

    /// Merge another version vector into this one (pointwise max).
    pub fn merge(&mut self, other: &VersionVector) {
        for (region, ts) in &other.entries {
            self.advance(region, *ts);
        }
    }

    /// Get all entries.
    pub fn entries(&self) -> &BTreeMap<String, HlcTimestamp> {
        &self.entries
    }

    /// Number of regions tracked.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for VersionVector {
    fn default() -> Self {
        Self::new()
    }
}

/// A replicated event carrying CRDT metadata for conflict resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicatedEvent {
    /// The event ID (UUID as string).
    pub event_id: String,
    /// HLC timestamp assigned at the origin region.
    pub hlc_timestamp: HlcTimestamp,
    /// Region ID where the event was originally ingested.
    pub origin_region: String,
    /// Serialized event data (the full Event struct as JSON).
    pub event_data: serde_json::Value,
}

/// Conflict resolution outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Accept: the incoming event should be applied.
    Accept,
    /// Skip: the incoming event is a duplicate or superseded.
    Skip,
}

/// Per-entity-type merge strategy for conflict resolution.
///
/// Controls how the CRDT resolver handles events for specific event types
/// (or prefixes). The default behavior is `AppendOnly`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Accept all events (default, current G-Set behavior).
    AppendOnly,
    /// Newer HLC timestamp wins for same entity+type. Earlier events are skipped.
    LastWriteWins,
    /// First event for entity+type wins. Subsequent events are rejected.
    FirstWriteWins,
}

/// CRDT-based conflict resolver for geo-replicated events.
///
/// Thread-safe: uses DashMap internally for the version vector.
///
/// Supports configurable per-entity-type merge strategies via
/// [`with_strategies`](Self::with_strategies). Strategy lookup uses prefix
/// matching: a strategy registered for `"config."` applies to
/// `"config.updated"`, `"config.deleted"`, etc. The most specific
/// (longest) prefix match wins. Unmatched types fall back to `AppendOnly`.
pub struct CrdtResolver {
    /// Per-region version vectors.
    version_vectors: DashMap<String, VersionVector>,
    /// Set of event IDs already seen (deduplication).
    seen_events: DashMap<String, ()>,
    /// Per-event-type merge strategies (prefix → strategy).
    /// Sorted by prefix length descending for longest-match-first lookup.
    strategies: Vec<(String, MergeStrategy)>,
    /// entity+type → winning HLC timestamp, used for LWW and FWW tracking.
    entity_type_winners: DashMap<String, HlcTimestamp>,
}

impl CrdtResolver {
    /// Create a new CRDT resolver with default AppendOnly behavior.
    pub fn new() -> Self {
        Self {
            version_vectors: DashMap::new(),
            seen_events: DashMap::new(),
            strategies: Vec::new(),
            entity_type_winners: DashMap::new(),
        }
    }

    /// Create a CRDT resolver with per-event-type merge strategies.
    ///
    /// Each entry is `(prefix, strategy)`. Prefix matching is used:
    /// `"config."` matches `"config.updated"`. The longest matching
    /// prefix wins. Unmatched types use `AppendOnly`.
    pub fn with_strategies(strategies: Vec<(String, MergeStrategy)>) -> Self {
        let mut sorted = strategies;
        // Sort by prefix length descending for longest-match-first
        sorted.sort_by_key(|a| std::cmp::Reverse(a.0.len()));
        Self {
            version_vectors: DashMap::new(),
            seen_events: DashMap::new(),
            strategies: sorted,
            entity_type_winners: DashMap::new(),
        }
    }

    /// Look up the merge strategy for an event type (longest prefix match).
    fn strategy_for(&self, event_type: &str) -> &MergeStrategy {
        for (prefix, strategy) in &self.strategies {
            if event_type.starts_with(prefix.as_str()) {
                return strategy;
            }
        }
        // Static reference to default — avoids allocation
        &MergeStrategy::AppendOnly
    }

    /// Resolve whether an incoming replicated event should be accepted.
    ///
    /// Returns `Accept` if the event is new, `Skip` if it's a duplicate.
    /// Consults the per-event-type merge strategy when one is registered.
    pub fn resolve(&self, event: &ReplicatedEvent) -> ConflictResolution {
        // Dedup by event ID
        if self.seen_events.contains_key(&event.event_id) {
            return ConflictResolution::Skip;
        }

        // Check version vector for this region
        let is_new = self
            .version_vectors
            .get(&event.origin_region)
            .is_none_or(|vv| vv.is_new(&event.origin_region, &event.hlc_timestamp));

        if !is_new {
            return ConflictResolution::Skip;
        }

        // Extract event_type and entity_id from event_data for strategy lookup
        let event_type = event
            .event_data
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let entity_id = event
            .event_data
            .get("entity_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match self.strategy_for(event_type) {
            MergeStrategy::AppendOnly => ConflictResolution::Accept,
            MergeStrategy::LastWriteWins => {
                let key = format!("{entity_id}\x00{event_type}");
                match self.entity_type_winners.get(&key) {
                    Some(existing) if event.hlc_timestamp <= *existing => ConflictResolution::Skip,
                    _ => ConflictResolution::Accept,
                }
            }
            MergeStrategy::FirstWriteWins => {
                let key = format!("{entity_id}\x00{event_type}");
                if self.entity_type_winners.contains_key(&key) {
                    ConflictResolution::Skip
                } else {
                    ConflictResolution::Accept
                }
            }
        }
    }

    /// Mark an event as accepted: update version vector, dedup set, and strategy state.
    pub fn accept(&self, event: &ReplicatedEvent) {
        self.seen_events.insert(event.event_id.clone(), ());

        let mut vv = self
            .version_vectors
            .entry(event.origin_region.clone())
            .or_default();
        vv.advance(&event.origin_region, event.hlc_timestamp);

        // Track entity+type winner for LWW/FWW strategies
        let event_type = event
            .event_data
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let entity_id = event
            .event_data
            .get("entity_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !event_type.is_empty() && !entity_id.is_empty() {
            let key = format!("{entity_id}\x00{event_type}");
            self.entity_type_winners
                .entry(key)
                .and_modify(|existing| {
                    if event.hlc_timestamp > *existing {
                        *existing = event.hlc_timestamp;
                    }
                })
                .or_insert(event.hlc_timestamp);
        }
    }

    /// Resolve and accept in one step. Returns the resolution.
    pub fn resolve_and_accept(&self, event: &ReplicatedEvent) -> ConflictResolution {
        let resolution = self.resolve(event);
        if resolution == ConflictResolution::Accept {
            self.accept(event);
        }
        resolution
    }

    /// Get the version vector for a specific region.
    pub fn version_vector_for(&self, region_id: &str) -> Option<VersionVector> {
        self.version_vectors.get(region_id).map(|vv| vv.clone())
    }

    /// Get all version vectors (for status/debug).
    pub fn all_version_vectors(&self) -> BTreeMap<String, VersionVector> {
        self.version_vectors
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Merge a remote version vector into our local state.
    pub fn merge_version_vector(&self, region_id: &str, remote_vv: &VersionVector) {
        let mut vv = self
            .version_vectors
            .entry(region_id.to_string())
            .or_default();
        vv.merge(remote_vv);
    }

    /// Number of unique events seen.
    pub fn seen_count(&self) -> usize {
        self.seen_events.len()
    }
}

impl Default for CrdtResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Sort replicated events into deterministic total order.
///
/// This produces the same ordering on every node, regardless of arrival order.
pub fn deterministic_order(events: &mut [ReplicatedEvent]) {
    events.sort_by(|a, b| {
        a.hlc_timestamp
            .cmp(&b.hlc_timestamp)
            .then_with(|| a.event_id.cmp(&b.event_id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(
        id: &str,
        region: &str,
        physical_ms: u64,
        logical: u32,
        node_id: u32,
    ) -> ReplicatedEvent {
        ReplicatedEvent {
            event_id: id.to_string(),
            hlc_timestamp: HlcTimestamp::new(physical_ms, logical, node_id),
            origin_region: region.to_string(),
            event_data: serde_json::json!({"type": "test"}),
        }
    }

    #[test]
    fn test_version_vector_advance() {
        let mut vv = VersionVector::new();
        let ts1 = HlcTimestamp::new(100, 0, 1);
        let ts2 = HlcTimestamp::new(200, 0, 1);
        let ts_old = HlcTimestamp::new(50, 0, 1);

        assert!(vv.advance("us-east", ts1));
        assert!(vv.advance("us-east", ts2)); // newer
        assert!(!vv.advance("us-east", ts_old)); // older, rejected

        assert_eq!(vv.get("us-east").unwrap().physical_ms, 200);
    }

    #[test]
    fn test_version_vector_is_new() {
        let mut vv = VersionVector::new();
        let ts = HlcTimestamp::new(100, 0, 1);
        vv.advance("us-east", ts);

        assert!(!vv.is_new("us-east", &HlcTimestamp::new(50, 0, 1)));
        assert!(!vv.is_new("us-east", &HlcTimestamp::new(100, 0, 1)));
        assert!(vv.is_new("us-east", &HlcTimestamp::new(101, 0, 1)));
        assert!(vv.is_new("eu-west", &HlcTimestamp::new(1, 0, 1))); // unknown region
    }

    #[test]
    fn test_version_vector_merge() {
        let mut vv1 = VersionVector::new();
        vv1.advance("us-east", HlcTimestamp::new(100, 0, 1));
        vv1.advance("eu-west", HlcTimestamp::new(50, 0, 2));

        let mut vv2 = VersionVector::new();
        vv2.advance("us-east", HlcTimestamp::new(80, 0, 1));
        vv2.advance("eu-west", HlcTimestamp::new(120, 0, 2));
        vv2.advance("ap-east", HlcTimestamp::new(90, 0, 3));

        vv1.merge(&vv2);

        assert_eq!(vv1.get("us-east").unwrap().physical_ms, 100); // kept higher
        assert_eq!(vv1.get("eu-west").unwrap().physical_ms, 120); // took higher
        assert_eq!(vv1.get("ap-east").unwrap().physical_ms, 90); // new entry
        assert_eq!(vv1.len(), 3);
    }

    #[test]
    fn test_crdt_resolver_accept_new_event() {
        let resolver = CrdtResolver::new();
        let event = make_event("evt-1", "us-east", 100, 0, 1);

        assert_eq!(resolver.resolve(&event), ConflictResolution::Accept);
        resolver.accept(&event);
        assert_eq!(resolver.seen_count(), 1);
    }

    #[test]
    fn test_crdt_resolver_skip_duplicate() {
        let resolver = CrdtResolver::new();
        let event = make_event("evt-1", "us-east", 100, 0, 1);

        assert_eq!(
            resolver.resolve_and_accept(&event),
            ConflictResolution::Accept
        );
        assert_eq!(resolver.resolve(&event), ConflictResolution::Skip);
    }

    #[test]
    fn test_crdt_resolver_skip_old_version() {
        let resolver = CrdtResolver::new();
        let new_event = make_event("evt-2", "us-east", 200, 0, 1);
        let old_event = make_event("evt-1", "us-east", 100, 0, 1);

        resolver.resolve_and_accept(&new_event);
        // old_event has lower HLC than what we've seen from us-east
        assert_eq!(resolver.resolve(&old_event), ConflictResolution::Skip);
    }

    #[test]
    fn test_crdt_resolver_different_regions_independent() {
        let resolver = CrdtResolver::new();
        let us_event = make_event("evt-1", "us-east", 100, 0, 1);
        let eu_event = make_event("evt-2", "eu-west", 50, 0, 2);

        resolver.resolve_and_accept(&us_event);
        // eu-west at lower timestamp is still accepted (different region)
        assert_eq!(resolver.resolve(&eu_event), ConflictResolution::Accept);
    }

    #[test]
    fn test_deterministic_order() {
        let mut events = vec![
            make_event("evt-3", "ap-east", 100, 0, 3),
            make_event("evt-1", "us-east", 100, 0, 1),
            make_event("evt-2", "eu-west", 100, 0, 2),
        ];

        deterministic_order(&mut events);

        // Same physical time + logical: ordered by node_id, then event_id
        assert_eq!(events[0].event_id, "evt-1"); // node 1
        assert_eq!(events[1].event_id, "evt-2"); // node 2
        assert_eq!(events[2].event_id, "evt-3"); // node 3
    }

    #[test]
    fn test_deterministic_order_by_hlc() {
        let mut events = vec![
            make_event("evt-1", "us-east", 300, 0, 1),
            make_event("evt-2", "eu-west", 100, 0, 2),
            make_event("evt-3", "ap-east", 200, 0, 3),
        ];

        deterministic_order(&mut events);

        assert_eq!(events[0].event_id, "evt-2"); // 100ms
        assert_eq!(events[1].event_id, "evt-3"); // 200ms
        assert_eq!(events[2].event_id, "evt-1"); // 300ms
    }

    #[test]
    fn test_replicated_event_serialization() {
        let event = make_event("evt-1", "us-east", 1000, 5, 1);
        let json = serde_json::to_string(&event).unwrap();
        let parsed: ReplicatedEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.event_id, "evt-1");
        assert_eq!(parsed.origin_region, "us-east");
        assert_eq!(parsed.hlc_timestamp.physical_ms, 1000);
    }
}
