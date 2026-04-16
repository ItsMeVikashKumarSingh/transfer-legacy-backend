use axum::extract::{Extension, State};
use axum::{Json, http::HeaderMap};
use serde::{Deserialize, Serialize};

use crate::errors::{success, ApiError};
use crate::state::AppState;
use crate::middleware::rate_limit::require_idempotency;

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

pub async fn refresh(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<RefreshResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid))?;
    
    let res = crate::services::supabase::refresh_session(&config, &payload.refresh_token)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid))?;

    Ok(success(&rid, RefreshResponse {
        access_token: res.access_token,
        refresh_token: res.refresh_token,
        expires_in: res.expires_in,
    }))
}
