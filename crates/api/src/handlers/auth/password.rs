use axum::extract::{Extension, State};
use axum::{Json, http::HeaderMap};
use serde::{Deserialize, Serialize};

use crate::errors::{success, ApiError};
use crate::state::AppState;
use crate::middleware::rate_limit::require_idempotency;

#[derive(Debug, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetConfirmRequest {
    pub access_token: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct PasswordResetResponse {
    pub status: &'static str,
}

pub async fn password_reset_request(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<PasswordResetRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<PasswordResetResponse>>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;
    crate::services::supabase::send_password_recovery(&state.config, &payload.email)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    Ok(success(&request_id, PasswordResetResponse { status: "ok" }))
}

pub async fn password_reset_confirm(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<PasswordResetConfirmRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<PasswordResetResponse>>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;
    crate::services::supabase::reset_password_with_token(&state.config, &payload.access_token, &payload.new_password)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    Ok(success(&request_id, PasswordResetResponse { status: "ok" }))
}
