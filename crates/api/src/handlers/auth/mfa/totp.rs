use axum::extract::{Extension, State};
use axum::{http::HeaderMap, Json};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

use crate::db::queries::mfa::{fetch_totp_secret, insert_totp_factor};
use crate::errors::{success, ApiError};
use crate::middleware::rate_limit::require_idempotency;
use crate::state::AppState;
use transfer_legacy_crypto_core::aead::{decrypt, encrypt};
use transfer_legacy_shared_types::{CURRENT_CRYPTO_VERSION, CURRENT_SCHEMA_VERSION};

#[derive(Debug, Deserialize)]
pub struct TotpEnrollRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct TotpEnrollResponse {
    pub otpauth_url: String,
    pub backup_codes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TotpVerifyRequest {
    pub user_id: Uuid,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct TotpVerifyResponse {
    pub status: &'static str,
}

pub async fn totp_enroll(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<TotpEnrollRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<TotpEnrollResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    require_idempotency(&state, &headers).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid)
    })?;

    let secret = Secret::generate_secret();
    let secret_bytes = secret.to_bytes().map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes.clone(),
        Some("Transfer Legacy".into()),
        payload.user_id.to_string(),
    )
    .map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    let otpauth_url = totp.get_url();

    let key = URL_SAFE_NO_PAD
        .decode(config.server_aead_key_b64.as_str())
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;
    let aad = payload.user_id.as_bytes();
    let enc = encrypt(&key, &secret_bytes, aad).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    let mut secret_enc = enc.nonce;
    secret_enc.extend_from_slice(&enc.ciphertext);

    insert_totp_factor(
        &state.db,
        payload.user_id,
        secret_enc,
        CURRENT_CRYPTO_VERSION.as_str().to_string(),
        CURRENT_SCHEMA_VERSION,
    )
    .await
    .map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let backup_codes = generate_backup_codes();

    Ok(success(
        &rid,
        TotpEnrollResponse {
            otpauth_url,
            backup_codes,
        },
    ))
}

pub async fn totp_verify(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(payload): Json<TotpVerifyRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<TotpVerifyResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    let secret_enc = fetch_totp_secret(&state.db, payload.user_id)
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
    let aad = payload.user_id.as_bytes();
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
        payload.user_id.to_string(),
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

    Ok(success(&rid, TotpVerifyResponse { status: "ok" }))
}

fn generate_backup_codes() -> Vec<String> {
    let mut codes = Vec::new();
    for _ in 0..8 {
        let code = uuid::Uuid::new_v4().to_string().replace('-', "")[..10].to_string();
        codes.push(code);
    }
    codes
}
