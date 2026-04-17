//! [`ProjectionWorker`] — the builder and event loop for custom projections.
//!
//! See the crate-level docs for an end-to-end example.
//!
//! This module is only available with the `projection-worker` feature.

use futures_util::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    client::CoreClient,
    error::Error,
    types::Event,
    ws::{StreamItem, StreamMode, StreamedEvent},
};

/// Bounded state that the reducer reads and writes.
///
/// Implementors need `Default` (for fresh workers) and `Send + Sync + 'static`
/// (for the worker's tokio task). `Serialize + DeserializeOwned` are required
/// for optional state push-back to Core.
pub trait WorkerState: Default + Send + Sync + 'static + Serialize + DeserializeOwned {}

impl<T> WorkerState for T where T: Default + Send + Sync + 'static + Serialize + DeserializeOwned {}

/// Reducer closure signature: mutate `state` based on `event`, return an error
/// to abort the worker. Returning `Ok(())` on unknown event types is expected
/// (the reducer is authoritative about what it cares about).
pub type Reducer<S> = dyn FnMut(&mut S, &Event) -> Result<(), Error> + Send + 'static;

/// Builder for a [`ProjectionWorker`].
///
/// Create via [`ProjectionWorker::builder`], chain the setters, then call
/// [`Self::build`] to validate and produce the worker.
pub struct ProjectionWorkerBuilder<S: WorkerState> {
    core: CoreClient,
    name: Option<String>,
    event_types: Vec<String>,
    reducer: Option<Box<Reducer<S>>>,
    checkpoint_interval: u64,
}

impl<S: WorkerState> ProjectionWorkerBuilder<S> {
    fn new(core: CoreClient) -> Self {
        Self {
            core,
            name: None,
            event_types: Vec::new(),
            reducer: None,
            checkpoint_interval: 100,
        }
    }

    /// Unique consumer name. Required. Used both for Core's durable-consumer
    /// registry and for disambiguating multiple workers against the same Core.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Event-type prefix filters (e.g. `["asset.", "trade."]`). Core applies
    /// these server-side during replay and live delivery.
    pub fn event_types(mut self, types: &[&str]) -> Self {
        self.event_types = types.iter().map(|s| (*s).to_string()).collect();
        self
    }

    /// Domain-specific reduction function. Required. Runs once per event, in
    /// WAL order. Errors abort the worker.
    pub fn reducer<F>(mut self, f: F) -> Self
    where
        F: FnMut(&mut S, &Event) -> Result<(), Error> + Send + 'static,
    {
        self.reducer = Some(Box::new(f));
        self
    }

    /// Save a checkpoint after every `n` processed events. Lower = more
    /// frequent restart-resume, higher = less Core traffic. Default 100.
    pub fn checkpoint_interval(mut self, n: u64) -> Self {
        self.checkpoint_interval = n.max(1);
        self
    }

    /// Validate configuration and produce the worker.
    pub fn build(self) -> Result<ProjectionWorker<S>, Error> {
        let name = self
            .name
            .ok_or_else(|| Error::Config("ProjectionWorker requires a name".into()))?;
        let reducer = self
            .reducer
            .ok_or_else(|| Error::Config("ProjectionWorker requires a reducer".into()))?;
        Ok(ProjectionWorker {
            core: self.core,
            name,
            event_types: self.event_types,
            reducer,
            checkpoint_interval: self.checkpoint_interval,
            state: Arc::new(RwLock::new(S::default())),
            last_applied_version_by_entity: HashMap::new(),
        })
    }
}

/// A projection worker.
///
/// Constructed via [`Self::builder`], consumed by `start()` (US-005) which
/// moves the worker into a spawned task.
pub struct ProjectionWorker<S: WorkerState> {
    pub(crate) core: CoreClient,
    pub(crate) name: String,
    // Used by `start()` in US-005 to open a filtered WS subscription.
    #[allow(dead_code)]
    pub(crate) event_types: Vec<String>,
    pub(crate) reducer: Box<Reducer<S>>,
    pub(crate) checkpoint_interval: u64,
    pub(crate) state: Arc<RwLock<S>>,
    pub(crate) last_applied_version_by_entity: HashMap<String, i64>,
}

impl<S: WorkerState> ProjectionWorker<S> {
    /// Start building a new worker bound to `core`.
    pub fn builder(core: CoreClient) -> ProjectionWorkerBuilder<S> {
        ProjectionWorkerBuilder::new(core)
    }

    /// Consume a stream of [`StreamItem`]s, applying each event to state.
    ///
    /// Handles:
    /// - per-entity version-based dedup (skips events with `version <=` last applied)
    /// - periodic checkpoint via `CoreClient::save_checkpoint` every
    ///   `checkpoint_interval` replay events
    /// - transition from replay → live (stops checkpointing until next replay)
    /// - propagating reducer errors upward
    ///
    /// Returns `Ok(())` when the stream ends cleanly, or the underlying error.
    /// This method is the pure event-processing loop; [`Self::start`] (US-005)
    /// wires it to a real WebSocket and manages reconnection.
    pub async fn run_with_stream<St>(&mut self, mut stream: St) -> Result<(), Error>
    where
        St: Stream<Item = Result<StreamItem, Error>> + Unpin,
    {
        let mut events_since_checkpoint: u64 = 0;
        let mut last_replay_position: Option<u64> = None;

        while let Some(item) = stream.next().await {
            match item? {
                StreamItem::Event(streamed) => {
                    if !self.apply_event(&streamed).await? {
                        continue;
                    }
                    if streamed.mode == StreamMode::Replay {
                        if let Some(pos) = streamed.position {
                            last_replay_position = Some(pos);
                        }
                        events_since_checkpoint += 1;
                        if events_since_checkpoint >= self.checkpoint_interval {
                            if let Some(pos) = last_replay_position {
                                self.core.save_checkpoint(&self.name, pos).await?;
                                tracing::debug!(
                                    worker = %self.name,
                                    position = pos,
                                    "checkpoint saved"
                                );
                                events_since_checkpoint = 0;
                            }
                        }
                    }
                }
                StreamItem::ReplayComplete { replayed } => {
                    // Final checkpoint at the end of replay so we don't reprocess
                    // the tail on the next cold start.
                    if let Some(pos) = last_replay_position {
                        self.core.save_checkpoint(&self.name, pos).await?;
                        events_since_checkpoint = 0;
                    }
                    tracing::info!(
                        worker = %self.name,
                        replayed,
                        "replay complete, entering live mode"
                    );
                }
                StreamItem::Lagged { missed } => {
                    tracing::warn!(
                        worker = %self.name,
                        missed,
                        "server broadcast lagged — events may have been dropped"
                    );
                }
            }
        }
        Ok(())
    }

    /// Apply a single event to state with per-entity version dedup.
    /// Returns true if the event was applied, false if skipped (duplicate).
    async fn apply_event(&mut self, streamed: &StreamedEvent) -> Result<bool, Error> {
        let entity_id = streamed.event.entity_id.clone();
        let version = streamed.event.version.unwrap_or(0);

        if let Some(&last) = self.last_applied_version_by_entity.get(&entity_id) {
            if version > 0 && version <= last {
                tracing::trace!(
                    worker = %self.name,
                    entity_id = %entity_id,
                    version,
                    last_applied = last,
                    "skipping duplicate event"
                );
                return Ok(false);
            }
        }

        {
            let mut state = self.state.write().await;
            (self.reducer)(&mut *state, &streamed.event)?;
        }

        if version > 0 {
            self.last_applied_version_by_entity.insert(entity_id, version);
        }
        Ok(true)
    }

    /// Name of this worker (the Core consumer_id).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Access the in-progress state. Mostly useful in tests; production code
    /// should use `ProjectionHandle::get_state` (US-005).
    pub fn state(&self) -> Arc<RwLock<S>> {
        Arc::clone(&self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws::{StreamItem, StreamMode, StreamedEvent};
    use futures_util::stream;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_event(entity: &str, event_type: &str, version: i64) -> Event {
        Event {
            id: format!("evt-{entity}-{version}"),
            event_type: event_type.into(),
            entity_id: entity.into(),
            payload: json!({}),
            metadata: json!({}),
            timestamp: "2026-04-17T00:00:00Z".into(),
            version: Some(version),
            tenant_id: None,
        }
    }

    fn replay(position: u64, event: Event) -> Result<StreamItem, Error> {
        Ok(StreamItem::Event(StreamedEvent {
            position: Some(position),
            event,
            mode: StreamMode::Replay,
        }))
    }

    fn live(event: Event) -> Result<StreamItem, Error> {
        Ok(StreamItem::Event(StreamedEvent {
            position: None,
            event,
            mode: StreamMode::Live,
        }))
    }

    async fn make_core(server: &MockServer) -> CoreClient {
        CoreClient::new(&server.uri(), "test").unwrap()
    }

    async fn mock_ack_endpoint(server: &MockServer) -> Arc<std::sync::atomic::AtomicU64> {
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let counter_clone = Arc::clone(&counter);
        Mock::given(method("POST"))
            .and(path("/api/v1/consumers/test-worker/ack"))
            .respond_with(move |_: &wiremock::Request| {
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({"status": "ok"}))
            })
            .mount(server)
            .await;
        counter
    }

    #[test]
    fn builder_requires_name_and_reducer() {
        let dummy = CoreClient::new("http://localhost:1", "k").unwrap();
        let err = ProjectionWorker::<Vec<String>>::builder(dummy.clone())
            .reducer(|_, _| Ok(()))
            .build()
            .err()
            .unwrap();
        assert!(matches!(err, Error::Config(_)));

        let err = ProjectionWorker::<Vec<String>>::builder(dummy)
            .name("w")
            .build()
            .err()
            .unwrap();
        assert!(matches!(err, Error::Config(_)));
    }

    #[test]
    fn builder_defaults_checkpoint_interval_to_100() {
        let dummy = CoreClient::new("http://localhost:1", "k").unwrap();
        let worker = ProjectionWorker::<Vec<String>>::builder(dummy)
            .name("w")
            .reducer(|_, _| Ok(()))
            .build()
            .unwrap();
        assert_eq!(worker.checkpoint_interval, 100);
    }

    #[test]
    fn builder_clamps_checkpoint_interval_above_zero() {
        let dummy = CoreClient::new("http://localhost:1", "k").unwrap();
        let worker = ProjectionWorker::<Vec<String>>::builder(dummy)
            .name("w")
            .reducer(|_, _| Ok(()))
            .checkpoint_interval(0)
            .build()
            .unwrap();
        assert_eq!(worker.checkpoint_interval, 1);
    }

    #[tokio::test]
    async fn runs_reducer_once_per_event() {
        let server = MockServer::start().await;
        let _acks = mock_ack_endpoint(&server).await;

        let core = make_core(&server).await;
        let mut worker = ProjectionWorker::<u64>::builder(core)
            .name("test-worker")
            .reducer(|state, _event| {
                *state += 1;
                Ok(())
            })
            .checkpoint_interval(1000) // above event count → no checkpoint fires
            .build()
            .unwrap();

        let stream = stream::iter(vec![
            replay(1, sample_event("a", "x", 1)),
            replay(2, sample_event("b", "x", 1)),
            replay(3, sample_event("c", "x", 1)),
        ]);

        worker.run_with_stream(stream).await.unwrap();
        let count = *worker.state().read().await;
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn checkpoint_fires_at_interval_boundary() {
        let server = MockServer::start().await;
        let acks = mock_ack_endpoint(&server).await;

        let core = make_core(&server).await;
        let mut worker = ProjectionWorker::<u64>::builder(core)
            .name("test-worker")
            .reducer(|state, _event| {
                *state += 1;
                Ok(())
            })
            .checkpoint_interval(5)
            .build()
            .unwrap();

        // Feed 12 replay events — expect checkpoints at 5 and 10 (2 calls).
        let events: Vec<_> = (1..=12)
            .map(|i| replay(i, sample_event(&format!("e{i}"), "x", 1)))
            .collect();
        worker
            .run_with_stream(stream::iter(events))
            .await
            .unwrap();

        let ack_count = acks.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(ack_count, 2, "expected 2 interval checkpoints, got {ack_count}");
    }

    #[tokio::test]
    async fn replay_complete_triggers_final_checkpoint() {
        let server = MockServer::start().await;
        let acks = mock_ack_endpoint(&server).await;

        let core = make_core(&server).await;
        let mut worker = ProjectionWorker::<u64>::builder(core)
            .name("test-worker")
            .reducer(|state, _event| {
                *state += 1;
                Ok(())
            })
            .checkpoint_interval(1000) // no interval checkpoints
            .build()
            .unwrap();

        let items = vec![
            replay(1, sample_event("a", "x", 1)),
            replay(2, sample_event("b", "x", 1)),
            Ok(StreamItem::ReplayComplete { replayed: 2 }),
        ];
        worker
            .run_with_stream(stream::iter(items))
            .await
            .unwrap();

        assert_eq!(acks.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn live_events_do_not_checkpoint() {
        let server = MockServer::start().await;
        let acks = mock_ack_endpoint(&server).await;

        let core = make_core(&server).await;
        let mut worker = ProjectionWorker::<u64>::builder(core)
            .name("test-worker")
            .reducer(|state, _event| {
                *state += 1;
                Ok(())
            })
            .checkpoint_interval(1)
            .build()
            .unwrap();

        // Only live events — no position info, no checkpointing.
        let items = vec![
            Ok(StreamItem::ReplayComplete { replayed: 0 }),
            live(sample_event("a", "x", 1)),
            live(sample_event("b", "x", 1)),
            live(sample_event("c", "x", 1)),
        ];
        worker
            .run_with_stream(stream::iter(items))
            .await
            .unwrap();

        // Zero interval-based checkpoints during live (replay_complete had no events to checkpoint).
        assert_eq!(
            acks.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "live events should not trigger checkpoints"
        );
    }

    #[tokio::test]
    async fn deduplicates_events_by_version() {
        let server = MockServer::start().await;
        let _acks = mock_ack_endpoint(&server).await;

        let core = make_core(&server).await;
        let mut worker = ProjectionWorker::<u64>::builder(core)
            .name("test-worker")
            .reducer(|state, _event| {
                *state += 1;
                Ok(())
            })
            .checkpoint_interval(1000)
            .build()
            .unwrap();

        // Same entity, versions 1, 2, 2 (dup), 1 (older dup), 3 → count = 3
        let items = vec![
            replay(1, sample_event("entity-1", "x", 1)),
            replay(2, sample_event("entity-1", "x", 2)),
            replay(3, sample_event("entity-1", "x", 2)), // dup
            replay(4, sample_event("entity-1", "x", 1)), // older dup
            replay(5, sample_event("entity-1", "x", 3)),
        ];
        worker
            .run_with_stream(stream::iter(items))
            .await
            .unwrap();
        assert_eq!(*worker.state().read().await, 3);
    }

    #[tokio::test]
    async fn dedup_is_per_entity() {
        let server = MockServer::start().await;
        let _acks = mock_ack_endpoint(&server).await;

        let core = make_core(&server).await;
        let mut worker = ProjectionWorker::<u64>::builder(core)
            .name("test-worker")
            .reducer(|state, _event| {
                *state += 1;
                Ok(())
            })
            .checkpoint_interval(1000)
            .build()
            .unwrap();

        // Two different entities at version 1 — both counted.
        let items = vec![
            replay(1, sample_event("entity-a", "x", 1)),
            replay(2, sample_event("entity-b", "x", 1)),
            replay(3, sample_event("entity-a", "x", 1)), // dup of a, skip
            replay(4, sample_event("entity-b", "x", 2)), // new
        ];
        worker
            .run_with_stream(stream::iter(items))
            .await
            .unwrap();
        assert_eq!(*worker.state().read().await, 3);
    }

    #[tokio::test]
    async fn reducer_error_aborts_loop() {
        let server = MockServer::start().await;
        let _acks = mock_ack_endpoint(&server).await;

        let core = make_core(&server).await;
        let mut worker = ProjectionWorker::<u64>::builder(core)
            .name("test-worker")
            .reducer(|_state, event| {
                if event.entity_id == "bad" {
                    Err(Error::Config("nope".into()))
                } else {
                    Ok(())
                }
            })
            .checkpoint_interval(1000)
            .build()
            .unwrap();

        let items = vec![
            replay(1, sample_event("good", "x", 1)),
            replay(2, sample_event("bad", "x", 1)),
            replay(3, sample_event("unreached", "x", 1)),
        ];
        let err = worker
            .run_with_stream(stream::iter(items))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Config(_)));
    }
}
