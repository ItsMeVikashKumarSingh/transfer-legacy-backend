use axum::extract::{Extension, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Html;
use axum::Json;
use serde_json::json;

use crate::errors::ApiError;
use crate::state::AppState;

pub async fn openapi_json(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
) -> Result<Json<serde_json::Value>, ApiError> {
    crate::middleware::internal_auth::ensure_internal_access(&state, &headers)
        .map_err(|_| ApiError::app(transfer_legacy_shared_types::AppError::Unauthorized))?;

    let spec = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Transfer Legacy API",
            "version": "1.0.0"
        },
        "servers": [{ "url": "/v1" }],
        "paths": {
            "/health": { "get": { "summary": "Health check" } },
            "/server-capabilities": { "get": { "summary": "Server capabilities" } },
            "/ops/reviews": { "get": { "summary": "List manual reviews" } },
            "/ops/reviews/{review_id}": { "get": { "summary": "Get manual review" } },
            "/ops/reviews/{review_id}/decision": { "post": { "summary": "Apply dual-sign decision" } },
            "/audit/chain": { "get": { "summary": "Get audit chain verification" } }
        },
        "x-request-id": crate::middleware::request_id::request_id_string(&request_id)
    });
    Ok(Json(spec))
}

pub async fn docs_ui(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Html<&'static str>, StatusCode> {
    crate::middleware::internal_auth::ensure_internal_access(&state, &headers)?;
    Ok(Html(r#"<!doctype html>
<html>
  <head><title>Transfer Legacy API Docs</title></head>
  <body>
    <h1>Transfer Legacy API Docs</h1>
    <p>Fetch the OpenAPI spec at <code>/v1/openapi.json</code>.</p>
  </body>
</html>"#))
}
