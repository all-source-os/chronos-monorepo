use allframe::hyper::{HeaderMap, StatusCode};

use super::BoxResponse;
use crate::AppState;

pub fn config(headers: &HeaderMap) -> BoxResponse {
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("registry.all-source.xyz");

    let scheme = if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
        "http"
    } else {
        "https"
    };

    let body = allframe::serde_json::json!({
        "dl": format!("{scheme}://{host}/cargo/crates/{{crate}}/{{version}}/download"),
        "api": format!("{scheme}://{host}/cargo")
    });

    super::json_response(StatusCode::OK, &body.to_string())
}

pub async fn index(state: &AppState, path: &str) -> BoxResponse {
    match state.storage.read_file(&format!("cargo/{path}")).await {
        Some(data) => super::text_response(StatusCode::OK, &String::from_utf8_lossy(&data)),
        None => super::json_response(StatusCode::NOT_FOUND, r#"{"error":"crate not found"}"#),
    }
}

/// Download path arrives as `{crate}/{version}/download` (prefix already stripped).
pub async fn download(state: &AppState, rest: &str) -> BoxResponse {
    let path = format!("cargo/crates/{rest}");
    match state.storage.read_file(&path).await {
        Some(data) => super::bytes_response(StatusCode::OK, "application/x-tar", data),
        None => super::json_response(StatusCode::NOT_FOUND, r#"{"error":"version not found"}"#),
    }
}
