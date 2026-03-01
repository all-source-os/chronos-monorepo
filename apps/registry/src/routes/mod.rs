pub mod cargo;
pub mod go;
pub mod health;
pub mod npm;
pub mod pypi;
pub mod upload;

use allframe::hyper::body::{Bytes, Incoming};
use allframe::hyper::{Method, Request, Response, StatusCode};
use http_body_util::Full;
use std::convert::Infallible;

use crate::AppState;

type BoxResponse = Response<Full<Bytes>>;

pub fn json_response(status: StatusCode, body: &str) -> BoxResponse {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

pub fn bytes_response(status: StatusCode, content_type: &str, data: Vec<u8>) -> BoxResponse {
    Response::builder()
        .status(status)
        .header("content-type", content_type)
        .body(Full::new(Bytes::from(data)))
        .unwrap()
}

pub fn html_response(status: StatusCode, body: String) -> BoxResponse {
    Response::builder()
        .status(status)
        .header("content-type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

pub fn text_response(status: StatusCode, body: &str) -> BoxResponse {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

fn not_found() -> BoxResponse {
    json_response(StatusCode::NOT_FOUND, r#"{"error":"not found"}"#)
}

fn unauthorized() -> BoxResponse {
    json_response(StatusCode::UNAUTHORIZED, r#"{"error":"unauthorized"}"#)
}

pub async fn dispatch(
    state: AppState,
    req: Request<Incoming>,
) -> Result<BoxResponse, Infallible> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    // Health — always public, no auth
    if path == "/health" {
        return Ok(health::handle());
    }

    // Upload — deploy token auth required
    if path.starts_with("/upload/") && method == Method::POST {
        if !state.auth.verify_deploy_token(req.headers()) {
            return Ok(unauthorized());
        }
        return Ok(upload::handle(state, req, &path).await);
    }

    // Download routes — optional token auth
    if !state.auth.verify_download_token(req.headers()) {
        return Ok(unauthorized());
    }

    // Cargo sparse registry
    if path == "/cargo/config.json" {
        return Ok(cargo::config(req.headers()));
    }
    if let Some(rest) = path.strip_prefix("/cargo/crates/")
        && rest.ends_with("/download")
    {
        return Ok(cargo::download(&state, rest).await);
    }
    if let Some(rest) = path.strip_prefix("/cargo/") {
        return Ok(cargo::index(&state, rest).await);
    }

    // Go module proxy (GOPROXY protocol)
    if let Some(rest) = path.strip_prefix("/go/") {
        return Ok(go::handle(&state, rest).await);
    }

    // npm registry
    if let Some(rest) = path.strip_prefix("/npm/") {
        return Ok(npm::handle(&state, rest).await);
    }

    // PyPI Simple API
    if path == "/pypi/simple/" || path == "/pypi/simple" {
        return Ok(pypi::simple_index(&state).await);
    }
    if let Some(rest) = path.strip_prefix("/pypi/simple/") {
        let project = rest.trim_end_matches('/');
        return Ok(pypi::project_index(&state, project).await);
    }
    if let Some(rest) = path.strip_prefix("/pypi/files/") {
        return Ok(pypi::download_file(&state, rest).await);
    }

    Ok(not_found())
}
