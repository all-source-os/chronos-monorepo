//! Server-Sent Events endpoint backed by `CoreBackend::subscribe()`.
//!
//! Consumers are htmx partials using `hx-ext="sse"` + `sse-connect=...`.
//! We emit one `message` event per backend change plus a heartbeat every
//! 15s so reverse proxies don't reap the connection during idle periods.
//!
//! The SSE body is intentionally minimal — just a `{}` JSON payload — so
//! htmx uses it as a pure refresh trigger. Partials re-pull state from
//! their own endpoints; SSE is only a notification channel.

use std::{convert::Infallible, time::Duration};

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::Stream;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::infrastructure::backend::SubscribeError;

use super::super::state::AppState;

pub async fn events_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let backend = state.repo.backend_arc().clone();

    // Bridge the broadcast receiver into an unbounded mpsc so we can hand
    // axum a Stream without fighting with Send bounds. Unbounded is safe:
    // the upstream Core broadcast channel already caps capacity.
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::spawn(async move {
        let mut sub = backend.subscribe();
        // Nudge newly-connected clients so they do an immediate refresh.
        let _ = tx.send(Ok::<Event, Infallible>(Event::default().data("{}")));

        while let Ok(_) | Err(SubscribeError::Lagged(_)) = sub.recv_change().await {
            // Any change (or a lag notification) nudges the client to refresh.
            if tx.send(Ok(Event::default().data("{}"))).is_err() {
                break;
            }
        }
    });

    let stream = UnboundedReceiverStream::new(rx);

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}
