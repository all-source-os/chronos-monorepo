use allframe::hyper::StatusCode;

use super::BoxResponse;

pub fn handle() -> BoxResponse {
    super::json_response(
        StatusCode::OK,
        r#"{"status":"healthy","service":"allsource-registry"}"#,
    )
}
