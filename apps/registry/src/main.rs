use allframe::hyper::server::conn::http1;
use allframe::hyper::service::service_fn;
use allframe::tokio;
use allframe::tracing;
use hyper_util::rt::TokioIo;
use std::path::PathBuf;
use std::sync::Arc;

mod auth;
mod routes;
mod storage;

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<storage::Storage>,
    pub auth: Arc<auth::AuthConfig>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("allsource_registry=info".parse().unwrap()),
        )
        .init();

    let data_dir = std::env::var("REGISTRY_DATA_DIR")
        .unwrap_or_else(|_| "/app/data/registry".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3901);

    let storage = Arc::new(storage::Storage::new(PathBuf::from(&data_dir)));
    let auth = Arc::new(auth::AuthConfig::from_env());

    let state = AppState { storage, auth };

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("failed to bind");

    tracing::info!("AllSource Registry listening on 0.0.0.0:{port}");
    tracing::info!(data_dir, "serving artifacts from");

    loop {
        let (stream, addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::error!(error = %e, "accept failed");
                continue;
            }
        };

        let state = state.clone();
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let svc = service_fn(move |req| {
                let state = state.clone();
                async move { routes::dispatch(state, req).await }
            });

            if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
                tracing::debug!(addr = %addr, error = %e, "connection error");
            }
        });
    }
}
