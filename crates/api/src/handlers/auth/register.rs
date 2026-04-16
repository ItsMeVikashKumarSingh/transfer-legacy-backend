use axum::extract::{State, Extension};
use axum::{Json, http::HeaderMap};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rand::RngCore;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use redis::AsyncCommands;

use crate::errors::{success, ApiError, SuccessEnvelope};
use crate::middleware::aead_transport::{AeadJson, AeadResponse, wrap_response};
use crate::middleware::rate_limit::{require_idempotency, enforce_rate_limit};
use crate::state::AppState;
use transfer_legacy_crypto_core::opaque::{
    registration_start,
    registration_finish,
};
use transfer_legacy_shared_types::{CURRENT_CRYPTO_VERSION, CURRENT_SCHEMA_VERSION};
use crate::db::queries::auth::{insert_person_and_link, insert_opaque_record, OpaqueRecordRow};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct RegisterInitRequest {
    pub user_id: Uuid,
    pub registration_request: String,
    pub credential_identifier: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterInitResponse {
    pub session_id: Uuid,
    pub registration_response: String,
    pub server_nonce: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterFinishRequest {
    pub session_id: Uuid,
    pub registration_upload: String,
    pub ed25519_pubkey: String,
    pub x25519_pubkey: String,
    pub kyber768_pubkey: String,
    pub emk_blob: String,
    pub argon2_params: Value,
    pub enc_legal_name: String,
    pub enc_email: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterFinishResponse {
    pub user_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterSession {
    user_id: Uuid,
    credential_identifier: String,
}

pub async fn register_init(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<RegisterInitRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    require_idempotency(&state, &headers).await?;
    let rate_key = format!("register_init:{}", payload.user_id);
    enforce_rate_limit(&state, &rate_key, 10).await?;
    
    let credential_identifier = payload
        .credential_identifier
        .unwrap_or_else(|| payload.user_id.to_string());

    let (registration_response, _req) = registration_start(
        &state.opaque_setup,
        &payload.registration_request,
    )
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid))?;

    let session_id = Uuid::new_v4();
    let mut nonce_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let server_nonce = URL_SAFE_NO_PAD.encode(nonce_bytes);

    let session = RegisterSession {
        user_id: payload.user_id,
        credential_identifier,
    };
    let mut conn = state.redis.get_multiplexed_async_connection().await.map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;
    let key = format!("opaque:reg:{}", session_id);
    let value = serde_json::to_string(&session)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;
    let _: () = conn.set_ex(key, value, 300).await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let envelope = SuccessEnvelope {
        data: RegisterInitResponse {
            session_id,
            registration_response,
            server_nonce,
        },
        request_id: rid,
    };
    let aead = wrap_response(&state.config().await, &headers, &envelope)?;
    Ok(Json(aead))
}

pub async fn register_finish(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<RegisterFinishRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    require_idempotency(&state, &headers).await?;
    let mut conn = state.redis.get_multiplexed_async_connection().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;
    let key = format!("opaque:reg:{}", payload.session_id);
    let session_json: Option<String> = conn.get(&key).await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;
    let session_json = session_json.ok_or_else(|| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid))?;
    let session: RegisterSession = serde_json::from_str(&session_json)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let opaque_record = registration_finish(&state.opaque_setup, &payload.registration_upload)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid))?;

    let ed25519_pubkey = URL_SAFE_NO_PAD.decode(payload.ed25519_pubkey)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid))?;
    let x25519_pubkey = URL_SAFE_NO_PAD.decode(payload.x25519_pubkey)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid))?;
    let kyber768_pubkey = URL_SAFE_NO_PAD.decode(payload.kyber768_pubkey)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid))?;
    let emk_blob = URL_SAFE_NO_PAD.decode(payload.emk_blob)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid))?;
    let enc_legal_name = URL_SAFE_NO_PAD.decode(payload.enc_legal_name)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid))?;
    let enc_email = URL_SAFE_NO_PAD.decode(payload.enc_email)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid))?;

    let mut tx = state.db.begin().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let _person_id = insert_person_and_link(&mut tx, session.user_id, enc_legal_name, enc_email)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let row = OpaqueRecordRow {
        user_id: session.user_id,
        opaque_record,
        emk_blob,
        argon2_params: payload.argon2_params,
        ed25519_pubkey,
        x25519_pubkey,
        kyber768_pubkey,
        crypto_version: CURRENT_CRYPTO_VERSION.as_str().to_string(),
        schema_version: CURRENT_SCHEMA_VERSION,
    };
    insert_opaque_record(&mut tx, &row)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    tx.commit().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: RegisterFinishResponse { user_id: session.user_id },
        request_id: rid,
    };
    let aead = wrap_response(&config, &headers, &envelope)?;
    Ok(Json(aead))
}
