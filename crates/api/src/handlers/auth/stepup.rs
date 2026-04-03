use axum::extract::{Extension, State};
use axum::{Json, http::HeaderMap};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{Duration, Utc};

use crate::errors::{success, ApiError};
use crate::state::AppState;
use crate::db::queries::stepup::{create_stepup_challenge, fetch_stepup_challenge, consume_stepup_challenge};
use crate::db::queries::mfa::fetch_totp_secret;
use crate::middleware::rate_limit::require_idempotency;
use transfer_legacy_crypto_core::aead::decrypt;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use totp_rs::{Algorithm, TOTP};

#[derive(Debug, Deserialize)]
pub struct StepUpRequest {
    pub user_id: Uuid,
    pub action: String,
    pub challenge_type: String,
}

#[derive(Debug, Serialize)]
pub struct StepUpResponse {
    pub challenge_id: Uuid,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct StepUpVerifyRequest {
    pub challenge_id: Uuid,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct StepUpVerifyResponse {
    pub status: &'static str,
}

pub async fn stepup_request(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<StepUpRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<StepUpResponse>>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;
    let expires_at = Utc::now() + Duration::minutes(5);
    let challenge_id = create_stepup_challenge(&state.db, payload.user_id, &payload.challenge_type, &payload.action, expires_at)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    Ok(success(&request_id, StepUpResponse { challenge_id, expires_at }))
}

pub async fn stepup_verify(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<StepUpVerifyRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<StepUpVerifyResponse>>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;
    let challenge = fetch_stepup_challenge(&state.db, payload.challenge_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &request_id))?;

    if challenge.consumed_at.is_some() || challenge.expires_at < Utc::now() {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &request_id));
    }

    if challenge.challenge_type != "totp" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id));
    }

    let secret_enc = fetch_totp_secret(&state.db, challenge.user_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &request_id))?;
    if secret_enc.len() < 24 {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id));
    }
    let (nonce, ciphertext) = secret_enc.split_at(24);
    let key = URL_SAFE_NO_PAD
        .decode(state.config.server_aead_key_b64.as_str())
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;
    let aad = challenge.user_id.as_bytes();
    let secret = decrypt(&key, nonce, ciphertext, aad)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, secret, Some("Transfer Legacy".into()), challenge.user_id.to_string())
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;
    let valid = totp.check_current(&payload.code)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    if !valid {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &request_id));
    }

    consume_stepup_challenge(&state.db, challenge.challenge_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    Ok(success(&request_id, StepUpVerifyResponse { status: "ok" }))
}
