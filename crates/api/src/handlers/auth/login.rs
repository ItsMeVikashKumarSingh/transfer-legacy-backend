use axum::extract::{Extension, State};
use axum::{http::HeaderMap, Json};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::auth::fetch_opaque_record;
use crate::errors::{success, ApiError, SuccessEnvelope};
use crate::middleware::aead_transport::{wrap_response, AeadJson, AeadResponse};
use crate::middleware::rate_limit::{enforce_rate_limit, require_idempotency};
use crate::state::AppState;
use transfer_legacy_crypto_core::opaque::{
    deserialize_login_state, login_finish as opaque_login_finish, login_start,
    serialize_login_state,
};

#[derive(Debug, Deserialize)]
pub struct LoginInitRequest {
    pub user_id: Uuid,
    pub credential_request: String,
}

#[derive(Debug, Serialize)]
pub struct LoginInitResponse {
    pub session_id: Uuid,
    pub credential_response: String,
    pub server_nonce: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginFinishRequest {
    pub session_id: Uuid,
    pub credential_finalization: String,
}

#[derive(Debug, Serialize)]
pub struct LoginFinishResponse {
    pub user_id: Uuid,
    pub session_token: String,
    pub emk_blob: String,
    pub argon2_params: serde_json::Value,
    pub ed25519_pubkey: String,
    pub x25519_pubkey: String,
    pub kyber768_pubkey: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginSession {
    user_id: Uuid,
    state_b64: String,
}

pub async fn login_init(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<LoginInitRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    require_idempotency(&state, &headers).await?;
    let rate_key = format!("login_init:{}", payload.user_id);
    enforce_rate_limit(&state, &rate_key, 10)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::RateLimited, &rid)
        })?;
    let record = fetch_opaque_record(&state.db, payload.user_id)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid)
        })?;

    let (credential_response, server_state) = login_start(
        &state.opaque_setup,
        &payload.credential_request,
        &record.opaque_record,
    )
    .map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;

    let state_bytes = serialize_login_state(&server_state).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    let state_b64 = URL_SAFE_NO_PAD.encode(state_bytes);

    let session_id = Uuid::new_v4();
    let mut nonce_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let server_nonce = URL_SAFE_NO_PAD.encode(nonce_bytes);

    let session = LoginSession {
        user_id: payload.user_id,
        state_b64,
    };

    let mut conn = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;
    let key = format!("opaque:login:{}", session_id);
    let value = serde_json::to_string(&session).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    let _: () = conn.set_ex(key, value, 300).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let envelope = SuccessEnvelope {
        data: LoginInitResponse {
            session_id,
            credential_response,
            server_nonce,
        },
        request_id: rid,
    };
    let aead = wrap_response(&state.config().await, &headers, &envelope)?;
    Ok(Json(aead))
}

pub async fn login_finish(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<LoginFinishRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    require_idempotency(&state, &headers).await?;
    let mut conn = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;
    let key = format!("opaque:login:{}", payload.session_id);
    let session_json: Option<String> = conn.get(&key).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    let session_json = session_json.ok_or_else(|| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;
    let session: LoginSession = serde_json::from_str(&session_json).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let state_bytes = URL_SAFE_NO_PAD.decode(session.state_b64).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;
    let server_state = deserialize_login_state(&state_bytes).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    opaque_login_finish(server_state, &payload.credential_finalization).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid)
    })?;

    let record = fetch_opaque_record(&state.db, session.user_id)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid)
        })?;

    let session_token = crate::services::sessions::issue_session_token(&config, session.user_id)
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    let _: () = conn.del(&key).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let envelope = crate::errors::SuccessEnvelope {
        data: LoginFinishResponse {
            user_id: session.user_id,
            session_token,
            emk_blob: URL_SAFE_NO_PAD.encode(record.emk_blob),
            argon2_params: record.argon2_params,
            ed25519_pubkey: URL_SAFE_NO_PAD.encode(record.ed25519_pubkey),
            x25519_pubkey: URL_SAFE_NO_PAD.encode(record.x25519_pubkey),
            kyber768_pubkey: URL_SAFE_NO_PAD.encode(record.kyber768_pubkey),
        },
        request_id: rid,
    };
    let aead = wrap_response(&config, &headers, &envelope)?;
    Ok(Json(aead))
}
