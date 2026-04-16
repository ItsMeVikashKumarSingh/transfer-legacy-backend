use axum::extract::Extension;
use axum::Json;
use serde::Serialize;
use tower_http::request_id::RequestId;

use crate::errors::success;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

pub async fn health(Extension(request_id): Extension<RequestId>) -> Json<crate::errors::SuccessEnvelope<HealthResponse>> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let version = option_env!("GIT_SHA").unwrap_or("unknown");
    success(&rid, HealthResponse { status: "ok", version })
}
