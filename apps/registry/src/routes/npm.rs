use allframe::hyper::StatusCode;

use super::BoxResponse;
use crate::AppState;

pub async fn handle(state: &AppState, path: &str) -> BoxResponse {
    // npm paths (prefix "/npm/" already stripped):
    //   @{scope}/{name}            -> packument JSON (all versions)
    //   @{scope}/{name}/-/{tarball} -> tarball download

    if let Some(tarball_pos) = path.find("/-/") {
        let tarball = &path[tarball_pos + 3..];
        let pkg = &path[..tarball_pos];
        let storage_path = format!("npm/{pkg}/-/{tarball}");
        return match state.storage.read_file(&storage_path).await {
            Some(data) => super::bytes_response(StatusCode::OK, "application/octet-stream", data),
            None => super::json_response(StatusCode::NOT_FOUND, r#"{"error":"tarball not found"}"#),
        };
    }

    // Packument request — stored as packument.json inside the package directory
    let storage_path = format!("npm/{path}/packument.json");
    match state.storage.read_file(&storage_path).await {
        Some(data) => super::bytes_response(StatusCode::OK, "application/json", data),
        None => super::json_response(StatusCode::NOT_FOUND, r#"{"error":"package not found"}"#),
    }
}
