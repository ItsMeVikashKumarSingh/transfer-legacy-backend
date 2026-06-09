use axum::extract::{Extension, State};
use axum::{http::HeaderMap, Json};
use serde::{Deserialize, Serialize};

use crate::errors::{success, ApiError};
use crate::middleware::rate_limit::require_idempotency;
use crate::notifications::resend::NotificationTemplate;
use crate::state::AppState;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use transfer_legacy_crypto_core::aead::decrypt;
use transfer_legacy_crypto_core::opaque::registration_finish;
use transfer_legacy_shared_types::{CURRENT_CRYPTO_VERSION, CURRENT_SCHEMA_VERSION};
use crate::db::queries::auth::{update_opaque_record, OpaqueRecordRow};
use uuid::Uuid;
use redis::AsyncCommands;
use rand::RngCore;

#[derive(Debug, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetInitRequest {
    pub access_token: String,
    pub new_password: String,
    pub registration_request: String,
}

#[derive(Debug, Serialize)]
pub struct PasswordResetInitResponse {
    pub session_id: Uuid,
    pub registration_response: String,
    pub server_nonce: String,
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetConfirmRequest {
    pub session_id: Uuid,
    pub registration_upload: String,
    pub ed25519_pubkey: String,
    pub x25519_pubkey: String,
    pub kyber768_pubkey: String,
    pub emk_blob: String,
    pub argon2_params: serde_json::Value,
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

    require_idempotency(&state, &headers).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid)
    })?;

    // 1. Fetch user ID and encrypted legal name
    let user_info = sqlx::query_as::<_, (Uuid, Option<String>)>(
        "SELECT u.id, p.enc_legal_name 
         FROM auth.users u 
         LEFT JOIN auth_ext.persons p ON p.user_id = u.id 
         WHERE u.email = $1",
    )
    .bind(&payload.email)
    .fetch_optional(state.db())
    .await
    .map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    if let Some((user_id, enc_name)) = user_info {
        // 2. Decrypt name if present
        let owner_name = if let Some(enc) = enc_name {
            decrypt_name(&config.server_aead_key_b64, &enc, user_id)
                .unwrap_or_else(|_| "User".to_string())
        } else {
            "User".to_string()
        };

        // 3. Generate recovery link via Supabase Admin API
        let recovery_link =
            crate::services::supabase::generate_recovery_link(&config, &payload.email)
                .await
                .map_err(|_| {
                    ApiError::app_with_request_id(
                        transfer_legacy_shared_types::AppError::Internal,
                        &rid,
                    )
                })?;

        // 4. Send via Resend
        let template = NotificationTemplate::PasswordReset {
            owner_name,
            reset_url: recovery_link,
        };

        state
            .notify(user_id, &payload.email, template)
            .await
            .map_err(|_| {
                ApiError::app_with_request_id(
                    transfer_legacy_shared_types::AppError::Internal,
                    &rid,
                )
            })?;
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

pub async fn password_reset_init(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<PasswordResetInitRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<PasswordResetInitResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    require_idempotency(&state, &headers).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid)
    })?;

    // 1. Consume the token and reset password in Supabase
    let user_id = crate::services::supabase::reset_password_with_token(
        &config,
        &payload.access_token,
        &payload.new_password,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to verify token or reset password in Supabase: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid)
    })?;

    // 2. Initialize OPAQUE registration handshake on the server
    let (registration_response, _req) =
        transfer_legacy_crypto_core::opaque::registration_start(&state.opaque_setup, &payload.registration_request)
            .map_err(|_| {
                ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
            })?;

    let session_id = Uuid::new_v4();
    let mut nonce_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let server_nonce = URL_SAFE_NO_PAD.encode(nonce_bytes);

    // 3. Cache the user ID session in Redis for 5 minutes (300 seconds)
    let mut conn = state.redis_conn.clone();
    let redis_key = format!("opaque:reset:{}", session_id);
    let value = user_id.to_string();
    let _: () = conn.set_ex(redis_key, value, 300).await.map_err(|e| {
        tracing::error!("Failed to store reset session in Redis: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    Ok(success(
        &rid,
        PasswordResetInitResponse {
            session_id,
            registration_response,
            server_nonce,
        },
    ))
}

pub async fn password_reset_confirm(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<PasswordResetConfirmRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<PasswordResetResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    require_idempotency(&state, &headers).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid)
    })?;

    // 1. Fetch user ID from Redis
    let redis_key = format!("opaque:reset:{}", payload.session_id);
    let mut conn = state.redis_conn.clone();
    
    let user_id_str: Option<String> = conn.get(&redis_key).await.map_err(|e| {
        tracing::error!("Failed to fetch reset session from Redis: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let user_id_str = user_id_str.ok_or_else(|| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;

    let user_id = Uuid::parse_str(&user_id_str).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    // 2. Decode and finalize the OPAQUE credential registration record
    let opaque_record = registration_finish(&state.opaque_setup, &payload.registration_upload)
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;

    let ed25519_pubkey = URL_SAFE_NO_PAD
        .decode(payload.ed25519_pubkey)
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;
    let x25519_pubkey = URL_SAFE_NO_PAD.decode(payload.x25519_pubkey).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;
    let kyber768_pubkey = URL_SAFE_NO_PAD
        .decode(payload.kyber768_pubkey)
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;
    let emk_blob = URL_SAFE_NO_PAD.decode(payload.emk_blob).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;

    let mut tx = state.db.begin().await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let row = OpaqueRecordRow {
        user_id,
        opaque_record,
        emk_blob,
        argon2_params: payload.argon2_params,
        ed25519_pubkey,
        x25519_pubkey,
        kyber768_pubkey,
        crypto_version: CURRENT_CRYPTO_VERSION.as_str().to_string(),
        schema_version: CURRENT_SCHEMA_VERSION,
    };

    update_opaque_record(&mut tx, &row).await.map_err(|e| {
        tracing::error!("Failed to update OPAQUE record in database: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    tx.commit().await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    // 3. Clear Redis reset session
    let _: () = conn.del(&redis_key).await.map_err(|e| {
        tracing::warn!("Failed to delete reset session from Redis: {:?}", e);
    }).unwrap_or_default();

    Ok(success(&rid, PasswordResetResponse { status: "ok" }))
}
