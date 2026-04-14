//! Cross-process WAL tail for embedded mode.
//!
//! When two `cn` processes share one `.chronis/` directory (e.g.
//! `cn tui` in one terminal and `cn task create` in another), each holds
//! its own in-memory `EmbeddedCore`. Writes from process B never appear
//! in process A's in-memory state, even though they hit the shared WAL
//! on disk.
//!
//! This module closes that gap. It:
//!   1. Watches the WAL directory via `notify`.
//!   2. On every filesystem change, scans the WAL files for entries we
//!      haven't seen yet (tracked by event UUID).
//!   3. Replays new events into our in-process `EmbeddedCore` via
//!      `ingest_replicated`, which updates indexes and projections
//!      without re-writing WAL or re-validating schemas.
//!   4. Emits a `ChangeEvent` so subscribed consumers refresh.
//!
//! Note: the deduplication set lives in memory and grows with the WAL.
//! That's fine for chronis' interactive use case (a few thousand tasks
//! at most) but would need a high-water-mark cursor for large stores.

use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Duration};

use allsource_core::embedded::EmbeddedCore;
use notify::{RecursiveMode, Watcher};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use super::backend::ChangeEvent;

/// Run the WAL tail loop. Returns only on unrecoverable error.
pub async fn run_wal_tail(
    wal_dir: PathBuf,
    core: Arc<EmbeddedCore>,
    change_tx: broadcast::Sender<ChangeEvent>,
) {
    if !wal_dir.exists() {
        tracing::debug!("wal_tail: {wal_dir:?} does not exist; embedded WAL tail disabled");
        return;
    }

    // Seed the seen-set with whatever is currently in our in-memory store.
    // Otherwise the first scan would replay every existing event back into
    // the store, doubling counts.
    let mut seen: HashSet<Uuid> = match core.query(allsource_core::embedded::Query::new()).await {
        Ok(events) => events.into_iter().map(|e| e.id).collect(),
        Err(e) => {
            tracing::warn!("wal_tail: initial seed query failed ({e}); starting empty");
            HashSet::new()
        }
    };

    // notify is sync; bridge it into a tokio mpsc.
    let (fs_tx, mut fs_rx) = mpsc::unbounded_channel::<()>();
    let watcher_tx = fs_tx.clone();
    let mut watcher =
        match notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if res.is_ok() {
                let _ = watcher_tx.send(());
            }
        }) {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("wal_tail: cannot create watcher: {e}");
                return;
            }
        };

    if let Err(e) = watcher.watch(&wal_dir, RecursiveMode::NonRecursive) {
        tracing::warn!("wal_tail: watch({wal_dir:?}) failed: {e}");
        return;
    }

    tracing::info!("wal_tail: watching {wal_dir:?} for cross-process WAL writes");

    // Drop the original sender so the channel closes when the watcher dies.
    drop(fs_tx);

    loop {
        // Block until we get at least one filesystem nudge.
        if fs_rx.recv().await.is_none() {
            tracing::debug!("wal_tail: watcher channel closed, exiting");
            return;
        }

        // Coalesce a burst of events — WAL appends are noisy. Wait briefly
        // and drain everything that arrived in the same window.
        tokio::time::sleep(Duration::from_millis(150)).await;
        while fs_rx.try_recv().is_ok() {}

        // The WAL recovery is blocking I/O; offload it.
        let core_for_scan = Arc::clone(&core);
        let scan_result = tokio::task::spawn_blocking(
            move || -> Result<Vec<allsource_core::embedded::Event>, String> {
                let store = core_for_scan.inner();
                let wal = store
                    .wal()
                    .ok_or_else(|| "WAL not enabled on this EmbeddedCore".to_string())?;
                wal.recover().map_err(|e| e.to_string())
            },
        )
        .await;

        let events = match scan_result {
            Ok(Ok(events)) => events,
            Ok(Err(e)) => {
                tracing::warn!("wal_tail: WAL recover failed: {e}");
                continue;
            }
            Err(e) => {
                tracing::warn!("wal_tail: spawn_blocking join error: {e}");
                continue;
            }
        };

        // Replay only events we haven't applied yet.
        let mut new_count = 0usize;
        for event in &events {
            if !seen.insert(event.id) {
                continue;
            }
            new_count += 1;

            let entity_id = event.entity_id_str().to_string();
            let event_type = event.event_type_str().to_string();

            // ingest_replicated runs synchronously; offload to blocking pool.
            let event_clone = event.clone();
            let core_for_ingest = Arc::clone(&core);
            let ingest_result = tokio::task::spawn_blocking(move || {
                core_for_ingest.inner().ingest_replicated(&event_clone)
            })
            .await;

            match ingest_result {
                Ok(Ok(())) => {
                    let _ = change_tx.send(ChangeEvent {
                        entity_id,
                        event_type,
                    });
                }
                Ok(Err(e)) => {
                    tracing::warn!("wal_tail: ingest_replicated failed: {e}");
                }
                Err(e) => {
                    tracing::warn!("wal_tail: spawn_blocking ingest join error: {e}");
                }
            }
        }

        if new_count > 0 {
            tracing::debug!("wal_tail: replayed {new_count} new events from WAL");
        }
    }
}
