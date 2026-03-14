// Copyright 2024-2025 AllSource Team
// Licensed under the Business Source License 1.1 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     See LICENSE-BSL in the repository root
//
// Change Date: 2029-03-01
// Change License: Apache License, Version 2.0

//! Replication protocol message types for leader-follower WAL shipping.
//!
//! Uses newline-delimited JSON over TCP:
//! 1. Follower connects and sends `Subscribe { last_offset }`
//! 2. Leader streams `WalEntry { offset, data }` for each new WAL write
//! 3. Follower sends `Ack { offset }` to confirm receipt

use crate::infrastructure::persistence::wal::WALEntry;
use serde::{Deserialize, Serialize};

/// Messages sent from follower to leader.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FollowerMessage {
    /// Initial subscription request with the follower's last known WAL offset.
    Subscribe { last_offset: u64 },
    /// Acknowledgement that a WAL entry has been received and applied.
    Ack { offset: u64 },
}

/// Messages sent from leader to follower.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum LeaderMessage {
    /// A WAL entry to be replayed by the follower.
    WalEntry { offset: u64, data: WALEntry },
    /// Sent after all catch-up entries have been streamed.
    CaughtUp { current_offset: u64 },
    /// Signals the start of a Parquet snapshot transfer for catch-up.
    /// Sent when the follower's last_offset is older than the oldest WAL entry.
    SnapshotStart {
        /// List of Parquet file names that will be streamed.
        parquet_files: Vec<String>,
    },
    /// A chunk of a Parquet file being transferred during catch-up.
    /// File contents are sent as base64-encoded binary chunks.
    SnapshotChunk {
        /// The Parquet file name this chunk belongs to.
        filename: String,
        /// Base64-encoded binary data for this chunk.
        data: String,
        /// Byte offset within the file where this chunk starts.
        chunk_offset: u64,
        /// Whether this is the last chunk for this file.
        is_last: bool,
    },
    /// Signals the end of Parquet snapshot transfer.
    /// After this, the leader resumes normal WAL streaming from the given offset.
    SnapshotEnd {
        /// The WAL offset to resume streaming from after loading the snapshot.
        wal_offset_after_snapshot: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_follower_subscribe_serialization() {
        let msg = FollowerMessage::Subscribe { last_offset: 42 };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"subscribe\""));
        assert!(json.contains("\"last_offset\":42"));

        let parsed: FollowerMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            FollowerMessage::Subscribe { last_offset } => assert_eq!(last_offset, 42),
            FollowerMessage::Ack { .. } => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_follower_ack_serialization() {
        let msg = FollowerMessage::Ack { offset: 100 };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"ack\""));

        let parsed: FollowerMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            FollowerMessage::Ack { offset } => assert_eq!(offset, 100),
            FollowerMessage::Subscribe { .. } => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_leader_caught_up_serialization() {
        let msg = LeaderMessage::CaughtUp {
            current_offset: 500,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"caught_up\""));

        let parsed: LeaderMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            LeaderMessage::CaughtUp { current_offset } => assert_eq!(current_offset, 500),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_snapshot_start_serialization() {
        let msg = LeaderMessage::SnapshotStart {
            parquet_files: vec![
                "events-001.parquet".to_string(),
                "events-002.parquet".to_string(),
            ],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"snapshot_start\""));
        assert!(json.contains("events-001.parquet"));

        let parsed: LeaderMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            LeaderMessage::SnapshotStart { parquet_files } => {
                assert_eq!(parquet_files.len(), 2);
                assert_eq!(parquet_files[0], "events-001.parquet");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_snapshot_chunk_serialization() {
        let msg = LeaderMessage::SnapshotChunk {
            filename: "events-001.parquet".to_string(),
            data: "AQIDBA==".to_string(), // base64 for [1,2,3,4]
            chunk_offset: 0,
            is_last: true,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"snapshot_chunk\""));
        assert!(json.contains("\"is_last\":true"));

        let parsed: LeaderMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            LeaderMessage::SnapshotChunk {
                filename,
                chunk_offset,
                is_last,
                ..
            } => {
                assert_eq!(filename, "events-001.parquet");
                assert_eq!(chunk_offset, 0);
                assert!(is_last);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_snapshot_end_serialization() {
        let msg = LeaderMessage::SnapshotEnd {
            wal_offset_after_snapshot: 1500,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"snapshot_end\""));

        let parsed: LeaderMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            LeaderMessage::SnapshotEnd {
                wal_offset_after_snapshot,
            } => assert_eq!(wal_offset_after_snapshot, 1500),
            _ => panic!("wrong variant"),
        }
    }
}
