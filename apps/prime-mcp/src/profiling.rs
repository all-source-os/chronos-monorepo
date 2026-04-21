//! In-process CPU profiling endpoint (Phase 2b of the rust-perf skill).
//!
//! When built with `--features profiling`, exposes `/debug/pprof/profile?seconds=N`
//! returning a pprof protobuf consumable by `go tool pprof` and pprof.me.
//!
//! **Never ship this feature-enabled on a publicly-reachable port.** Gate at
//! network level (private IPv6 address / fly internal network) or add auth
//! before the route. Matches the pattern from
//! ~/.claude/skills/rust-perf/templates/pprof-endpoint-axum.rs.

use axum::Router;

#[cfg(feature = "profiling")]
mod inner {
    use std::time::Duration;

    use axum::{
        Router,
        extract::Query,
        http::{StatusCode, header},
        response::IntoResponse,
        routing::get,
    };
    use pprof::protos::Message as _;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct ProfileParams {
        seconds: Option<u64>,
    }

    async fn profile_handler(Query(p): Query<ProfileParams>) -> impl IntoResponse {
        let duration = Duration::from_secs(p.seconds.unwrap_or(30).min(120));
        let guard = match pprof::ProfilerGuardBuilder::default()
            .frequency(100)
            .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()
        {
            Ok(g) => g,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("pprof guard init failed: {e}"),
                )
                    .into_response();
            }
        };
        tokio::time::sleep(duration).await;
        let report = match guard.report().build() {
            Ok(r) => r,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("pprof report build failed: {e}"),
                )
                    .into_response();
            }
        };
        let profile = match report.pprof() {
            Ok(p) => p,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("pprof encode failed: {e}"),
                )
                    .into_response();
            }
        };
        let mut buf = Vec::new();
        if let Err(e) = profile.encode(&mut buf) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("protobuf encode failed: {e}"),
            )
                .into_response();
        }
        (
            [(header::CONTENT_TYPE, "application/octet-stream")],
            buf,
        )
            .into_response()
    }

    pub fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
        Router::new().route("/debug/pprof/profile", get(profile_handler))
    }
}

#[cfg(feature = "profiling")]
pub fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    inner::routes()
}

#[cfg(not(feature = "profiling"))]
pub fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
}
