use allframe::hyper::body::Incoming;
use allframe::hyper::{Request, StatusCode};
use allframe::serde_json;
use allframe::tracing;
use http_body_util::BodyExt;
use sha2::{Digest, Sha256};

use super::BoxResponse;
use crate::storage::Storage;
use crate::AppState;

/// `POST /upload/{protocol}/{name}/{version}`
///
/// Accepts raw artifact bytes in the request body.
/// Protocol determines how metadata is generated:
///   - cargo:  writes .crate + appends index line
///   - npm:    writes .tgz + updates packument
///   - pypi:   writes file + updates project index.html
///   - go:     writes .zip/.mod/.info + appends to version list
pub async fn handle(state: AppState, req: Request<Incoming>, path: &str) -> BoxResponse {
    // Parse path: /upload/{protocol}/{name...}/{version}
    let rest = path.strip_prefix("/upload/").unwrap_or("");
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.len() < 2 {
        return super::json_response(
            StatusCode::BAD_REQUEST,
            r#"{"error":"expected /upload/{protocol}/{name}/{version}"}"#,
        );
    }

    let protocol = parts[0];
    let name_version = parts[1];

    // Split name and version: last segment is version
    let (name, version) = match name_version.rsplit_once('/') {
        Some((n, v)) => (n, v),
        None => {
            return super::json_response(
                StatusCode::BAD_REQUEST,
                r#"{"error":"expected /upload/{protocol}/{name}/{version}"}"#,
            );
        }
    };

    // Read body
    let body = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            tracing::error!(error = %e, "failed to read upload body");
            return super::json_response(
                StatusCode::BAD_REQUEST,
                r#"{"error":"failed to read body"}"#,
            );
        }
    };

    if body.is_empty() {
        return super::json_response(
            StatusCode::BAD_REQUEST,
            r#"{"error":"empty body"}"#,
        );
    }

    let sha256 = hex_sha256(&body);
    tracing::info!(protocol, name, version, bytes = body.len(), sha256 = %sha256, "upload");

    let result = match protocol {
        "cargo" => handle_cargo(&state, name, version, &body, &sha256).await,
        "npm" => handle_npm(&state, name, version, &body, &sha256).await,
        "pypi" => handle_pypi(&state, name, version, &body, &sha256).await,
        "go" => handle_go(&state, name, version, &body).await,
        _ => {
            return super::json_response(
                StatusCode::BAD_REQUEST,
                r#"{"error":"unknown protocol, expected: cargo|npm|pypi|go"}"#,
            );
        }
    };

    match result {
        Ok(()) => super::json_response(
            StatusCode::OK,
            &serde_json::json!({
                "status": "uploaded",
                "protocol": protocol,
                "name": name,
                "version": version,
                "sha256": sha256,
            })
            .to_string(),
        ),
        Err(e) => {
            tracing::error!(error = %e, "upload failed");
            super::json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                r#"{"error":"upload failed"}"#,
            )
        }
    }
}

async fn handle_cargo(
    state: &AppState,
    name: &str,
    version: &str,
    data: &[u8],
    sha256: &str,
) -> std::io::Result<()> {
    // Write .crate tarball
    let artifact_path = format!("cargo/crates/{name}/{version}/download");
    state.storage.write_file(&artifact_path, data).await?;

    // Append index line (minimal — real metadata comes from cargo package)
    let index_path = Storage::cargo_index_path(name);
    let line = serde_json::json!({
        "name": name,
        "vers": version,
        "deps": [],
        "cksum": sha256,
        "features": {},
        "yanked": false,
        "v": 2
    });
    let mut entry = line.to_string();
    entry.push('\n');
    state
        .storage
        .append_file(&format!("cargo/{index_path}"), entry.as_bytes())
        .await?;

    Ok(())
}

async fn handle_npm(
    state: &AppState,
    name: &str,
    version: &str,
    data: &[u8],
    sha256: &str,
) -> std::io::Result<()> {
    // Derive tarball filename from scoped name: @allsource/client -> client-{version}.tgz
    let short_name = name.rsplit('/').next().unwrap_or(name);
    let tarball_name = format!("{short_name}-{version}.tgz");

    // Write tarball
    let tarball_path = format!("npm/{name}/-/{tarball_name}");
    state.storage.write_file(&tarball_path, data).await?;

    // Read or create packument (stored as packument.json to avoid dir collision)
    let packument_path = format!("npm/{name}/packument.json");
    let mut packument: serde_json::Value =
        match state.storage.read_file(&packument_path).await {
            Some(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                serde_json::json!({
                    "name": name,
                    "dist-tags": {},
                    "versions": {}
                })
            }),
            None => serde_json::json!({
                "name": name,
                "dist-tags": {},
                "versions": {}
            }),
        };

    // Add version entry
    let host = "registry.all-source.xyz";
    packument["dist-tags"]["latest"] = serde_json::json!(version);
    packument["versions"][version] = serde_json::json!({
        "name": name,
        "version": version,
        "dist": {
            "tarball": format!("https://{host}/npm/{name}/-/{tarball_name}"),
            "shasum": sha256,
        }
    });

    state
        .storage
        .write_file(&packument_path, packument.to_string().as_bytes())
        .await?;

    Ok(())
}

async fn handle_pypi(
    state: &AppState,
    name: &str,
    version: &str,
    data: &[u8],
    sha256: &str,
) -> std::io::Result<()> {
    // Normalize project name for PyPI (PEP 503)
    let normalized = name.to_lowercase();

    // Determine filename: use sdist naming convention
    let filename = format!("{normalized}-{version}.tar.gz");

    // Write artifact
    let file_path = format!("pypi/files/{filename}");
    state.storage.write_file(&file_path, data).await?;

    // Read existing index or start fresh
    let index_path = format!("pypi/simple/{normalized}/index.html");
    let mut html = match state.storage.read_file(&index_path).await {
        Some(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        None => "<!DOCTYPE html>\n<html><body>\n</body></html>\n".to_string(),
    };

    // Insert link before </body>
    let link = format!(
        "<a href=\"/pypi/files/{filename}#sha256={sha256}\">{filename}</a>\n"
    );
    if let Some(pos) = html.rfind("</body>") {
        html.insert_str(pos, &link);
    }

    state
        .storage
        .write_file(&index_path, html.as_bytes())
        .await?;

    Ok(())
}

async fn handle_go(
    state: &AppState,
    module: &str,
    version: &str,
    data: &[u8],
) -> std::io::Result<()> {
    let base = format!("go/{module}/@v");

    // Write .zip
    state
        .storage
        .write_file(&format!("{base}/{version}.zip"), data)
        .await?;

    // Write .info
    let info = serde_json::json!({
        "Version": version,
        "Time": chrono_now(),
    });
    state
        .storage
        .write_file(
            &format!("{base}/{version}.info"),
            info.to_string().as_bytes(),
        )
        .await?;

    // Append to version list
    let list_entry = format!("{version}\n");
    state
        .storage
        .append_file(&format!("{base}/list"), list_entry.as_bytes())
        .await?;

    // Write @latest
    state
        .storage
        .write_file(
            &format!("go/{module}/@latest"),
            info.to_string().as_bytes(),
        )
        .await?;

    Ok(())
}

fn hex_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{b:02x}")).collect()
}

fn chrono_now() -> String {
    // Simple ISO 8601 timestamp without chrono dependency
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    // Rough UTC formatting: good enough for Go .info files
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Days since 1970-01-01
    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let months_days: &[i64] = if is_leap(y) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1;
    for &md in months_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        m += 1;
    }
    let d = remaining + 1;
    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
