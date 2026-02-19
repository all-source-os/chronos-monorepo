use allframe::hyper::StatusCode;

use super::BoxResponse;
use crate::AppState;

pub async fn handle(state: &AppState, path: &str) -> BoxResponse {
    // GOPROXY protocol paths:
    //   {module}/@v/list         -> version list
    //   {module}/@v/{v}.info     -> version info JSON
    //   {module}/@v/{v}.mod      -> go.mod contents
    //   {module}/@v/{v}.zip      -> source zip
    //   {module}/@latest         -> latest version info

    if let Some(pos) = path.find("/@v/") {
        let _module = &path[..pos];
        let rest = &path[pos + 4..]; // after "/@v/"

        if rest == "list" {
            return serve_file(state, &format!("go/{path}"), "text/plain").await;
        }
        if rest.ends_with(".info") {
            return serve_file(state, &format!("go/{path}"), "application/json").await;
        }
        if rest.ends_with(".mod") {
            return serve_file(state, &format!("go/{path}"), "text/plain").await;
        }
        if rest.ends_with(".zip") {
            return serve_file(state, &format!("go/{path}"), "application/zip").await;
        }
    }

    if path.ends_with("/@latest") {
        return serve_file(state, &format!("go/{path}"), "application/json").await;
    }

    super::json_response(StatusCode::NOT_FOUND, r#"{"error":"not found"}"#)
}

async fn serve_file(state: &AppState, path: &str, content_type: &str) -> BoxResponse {
    match state.storage.read_file(path).await {
        Some(data) => super::bytes_response(StatusCode::OK, content_type, data),
        None => super::json_response(StatusCode::NOT_FOUND, r#"{"error":"not found"}"#),
    }
}
