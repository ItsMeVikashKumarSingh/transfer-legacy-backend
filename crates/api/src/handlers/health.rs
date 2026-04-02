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
    let version = option_env!("GIT_SHA").unwrap_or("unknown");
    success(&request_id, HealthResponse { status: "ok", version })
}
