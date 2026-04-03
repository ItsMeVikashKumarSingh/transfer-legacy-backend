use axum::extract::{Extension, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::errors::ApiError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct WebAuthnStartRequest {
    pub user_id: uuid::Uuid,
}

#[derive(Debug, Serialize)]
pub struct WebAuthnStartResponse {
    pub status: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct WebAuthnFinishRequest {
    pub user_id: uuid::Uuid,
}

#[derive(Debug, Serialize)]
pub struct WebAuthnFinishResponse {
    pub status: &'static str,
}

pub async fn register_start(
    State(_state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(_payload): Json<WebAuthnStartRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<WebAuthnStartResponse>>, ApiError> {
    Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))
}

pub async fn register_finish(
    State(_state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(_payload): Json<WebAuthnFinishRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<WebAuthnFinishResponse>>, ApiError> {
    Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))
}

pub async fn authenticate_start(
    State(_state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(_payload): Json<WebAuthnStartRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<WebAuthnStartResponse>>, ApiError> {
    Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))
}

pub async fn authenticate_finish(
    State(_state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(_payload): Json<WebAuthnFinishRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<WebAuthnFinishResponse>>, ApiError> {
    Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))
}
