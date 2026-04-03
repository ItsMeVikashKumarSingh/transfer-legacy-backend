use axum::extract::{Extension, State};
use axum::{Json, http::HeaderMap};
use serde::Serialize;

use crate::errors::{success, ApiError};
use crate::state::AppState;
use crate::middleware::rate_limit::require_idempotency;

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub status: &'static str,
}

pub async fn logout(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
) -> Result<Json<crate::errors::SuccessEnvelope<LogoutResponse>>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;
    let auth = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let token = auth.strip_prefix("Bearer ").unwrap_or("");

    if !token.is_empty() {
        let _ = crate::services::supabase::logout_session(&state.config, token).await;
    }

    Ok(success(&request_id, LogoutResponse { status: "ok" }))
}
