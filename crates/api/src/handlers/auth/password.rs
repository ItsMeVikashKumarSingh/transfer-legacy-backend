use axum::extract::{Extension, State};
use axum::{Json, http::HeaderMap};
use serde::{Deserialize, Serialize};

use crate::errors::{success, ApiError};
use crate::state::AppState;
use crate::middleware::rate_limit::require_idempotency;
use crate::notifications::resend::NotificationTemplate;
use transfer_legacy_crypto_core::aead::decrypt;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use uuid::Uuid;

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
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid))?;
    
    // 1. Fetch user ID and encrypted legal name
    let user_info = sqlx::query_as::<_, (Uuid, Option<String>)>(
        "SELECT u.id, p.enc_legal_name 
         FROM auth.users u 
         LEFT JOIN auth_ext.persons p ON p.user_id = u.id 
         WHERE u.email = $1"
    )
    .bind(&payload.email)
    .fetch_optional(state.db())
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    if let Some((user_id, enc_name)) = user_info {
        // 2. Decrypt name if present
        let owner_name = if let Some(enc) = enc_name {
            decrypt_name(&config.server_aead_key_b64, &enc, user_id)
                .unwrap_or_else(|_| "User".to_string())
        } else {
            "User".to_string()
        };

        // 3. Generate recovery link via Supabase Admin API
        let recovery_link = crate::services::supabase::generate_recovery_link(&config, &payload.email)
            .await
            .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

        // 4. Send via Resend
        let template = NotificationTemplate::PasswordReset {
            owner_name,
            reset_url: recovery_link,
        };

        state.notify(user_id, &payload.email, template)
            .await
            .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;
    }

    Ok(success(&rid, PasswordResetResponse { status: "ok" }))
}

fn decrypt_name(key_b64: &str, enc_b64: &str, user_id: Uuid) -> anyhow::Result<String> {
    let key_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(key_b64)?;
    let enc_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(enc_b64)?;
    
    if enc_bytes.len() < 24 {
        return Err(anyhow::anyhow!("Invalid encrypted data length"));
    }
    
    let (nonce, ciphertext) = enc_bytes.split_at(24);
    let aad = user_id.as_bytes();
    
    let name_bytes = decrypt(&key_bytes, nonce, ciphertext, aad)
        .map_err(|_| anyhow::anyhow!("Decryption failed"))?;
    Ok(String::from_utf8(name_bytes)?)
}

pub async fn password_reset_confirm(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<PasswordResetConfirmRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<PasswordResetResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid))?;
    
    crate::services::supabase::reset_password_with_token(&config, &payload.access_token, &payload.new_password)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    Ok(success(&rid, PasswordResetResponse { status: "ok" }))
}
