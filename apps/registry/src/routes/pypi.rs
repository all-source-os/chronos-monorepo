use allframe::hyper::StatusCode;

use super::BoxResponse;
use crate::AppState;

/// `GET /pypi/simple/` — HTML index listing all projects.
pub async fn simple_index(state: &AppState) -> BoxResponse {
    let projects = state.storage.list_dir("pypi/simple").await;
    let mut html = String::from("<!DOCTYPE html>\n<html><body>\n");
    for project in &projects {
        html.push_str(&format!(
            "<a href=\"/pypi/simple/{project}/\">{project}</a>\n"
        ));
    }
    html.push_str("</body></html>\n");
    super::html_response(StatusCode::OK, html)
}

/// `GET /pypi/simple/{project}/` — HTML with file links and sha256 fragments.
pub async fn project_index(state: &AppState, project: &str) -> BoxResponse {
    let path = format!("pypi/simple/{project}/index.html");
    match state.storage.read_file(&path).await {
        Some(data) => super::html_response(StatusCode::OK, String::from_utf8_lossy(&data).into()),
        None => {
            super::json_response(StatusCode::NOT_FOUND, r#"{"error":"project not found"}"#)
        }
    }
}

/// `GET /pypi/files/{filename}` — wheel or sdist download.
pub async fn download_file(state: &AppState, filename: &str) -> BoxResponse {
    let path = format!("pypi/files/{filename}");
    match state.storage.read_file(&path).await {
        Some(data) => super::bytes_response(StatusCode::OK, "application/octet-stream", data),
        None => super::json_response(StatusCode::NOT_FOUND, r#"{"error":"file not found"}"#),
    }
}
