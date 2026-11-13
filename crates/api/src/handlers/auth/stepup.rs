use axum::extract::{Extension, State};
use axum::{http::HeaderMap, Json};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::mfa::fetch_totp_secret;
use crate::db::queries::stepup::{
    consume_stepup_challenge, create_stepup_challenge, fetch_stepup_challenge,
};
use crate::errors::{success, ApiError};
use crate::middleware::rate_limit::require_idempotency;
use crate::state::AppState;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use totp_rs::{Algorithm, TOTP};
use transfer_legacy_crypto_core::aead::decrypt;

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
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    require_idempotency(&state, &headers).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid)
    })?;
    let expires_at = Utc::now() + Duration::minutes(5);
    let challenge_id = create_stepup_challenge(
        &state.db,
        payload.user_id,
        &payload.challenge_type,
        &payload.action,
        expires_at,
    )
    .await
    .map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    Ok(success(
        &rid,
        StepUpResponse {
            challenge_id,
            expires_at,
        },
    ))
}

pub async fn stepup_verify(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<StepUpVerifyRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<StepUpVerifyResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    require_idempotency(&state, &headers).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid)
    })?;
    let challenge = fetch_stepup_challenge(&state.db, payload.challenge_id)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid)
        })?;

    if challenge.consumed_at.is_some() || challenge.expires_at < Utc::now() {
        return Err(ApiError::app_with_request_id(
            transfer_legacy_shared_types::AppError::OtpExpired,
            &rid,
        ));
    }

    if challenge.challenge_type != "totp" {
        return Err(ApiError::app_with_request_id(
            transfer_legacy_shared_types::AppError::BadRequest,
            &rid,
        ));
    }

    let secret_enc = fetch_totp_secret(&state.db, challenge.user_id)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid)
        })?;
    if secret_enc.len() < 24 {
        return Err(ApiError::app_with_request_id(
            transfer_legacy_shared_types::AppError::Internal,
            &rid,
        ));
    }
    let (nonce, ciphertext) = secret_enc.split_at(24);
    let key = URL_SAFE_NO_PAD
        .decode(config.server_aead_key_b64.as_str())
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;
    let aad = challenge.user_id.as_bytes();
    let secret = decrypt(&key, nonce, ciphertext, aad).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret,
        Some("Transfer Legacy".into()),
        challenge.user_id.to_string(),
    )
    .map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    let valid = totp.check_current(&payload.code).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    if !valid {
        return Err(ApiError::app_with_request_id(
            transfer_legacy_shared_types::AppError::OtpInvalid,
            &rid,
        ));
    }

    consume_stepup_challenge(&state.db, challenge.challenge_id)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    Ok(success(&rid, StepUpVerifyResponse { status: "ok" }))
}
