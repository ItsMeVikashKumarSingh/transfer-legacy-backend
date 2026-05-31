use axum::extract::{Extension, State};
use axum::http::HeaderMap;
use axum::Json;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::mfa::{fetch_webauthn_credential, upsert_webauthn_factor};
use crate::errors::success;
use crate::errors::ApiError;
use crate::middleware::rate_limit::require_idempotency;
use crate::state::AppState;
use transfer_legacy_crypto_core::{hash::sha256, signatures::verify_ed25519};
use transfer_legacy_shared_types::{CURRENT_CRYPTO_VERSION, CURRENT_SCHEMA_VERSION};

#[derive(Debug, Deserialize)]
pub struct WebAuthnStartRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct WebAuthnStartResponse {
    pub challenge_id: Uuid,
    pub challenge_b64: String,
}

#[derive(Debug, Deserialize)]
pub struct WebAuthnFinishRequest {
    pub user_id: Uuid,
    pub challenge_id: Uuid,
    pub credential_id: String,
    pub public_key_b64: Option<String>,
    pub signature_b64: String,
    pub authenticator_data_b64: String,
    pub client_data_json_b64: String,
}

#[derive(Debug, Serialize)]
pub struct WebAuthnFinishResponse {
    pub status: &'static str,
}

pub async fn register_start(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<WebAuthnStartRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<WebAuthnStartResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    require_idempotency(&state, &headers).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid)
    })?;

    let challenge_id = Uuid::new_v4();
    let challenge_bytes = Uuid::new_v4().as_bytes().to_vec();
    let challenge_b64 = URL_SAFE_NO_PAD.encode(challenge_bytes);
    let key = format!("webauthn:register:{}", challenge_id);
    let value = serde_json::json!({
        "user_id": payload.user_id,
        "challenge_b64": challenge_b64,
    })
    .to_string();

    let mut conn = state.redis_conn.clone();
    let _: () = conn.set_ex(key, value, 300).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    Ok(success(
        &rid,
        WebAuthnStartResponse {
            challenge_id,
            challenge_b64,
        },
    ))
}

pub async fn register_finish(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<WebAuthnFinishRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<WebAuthnFinishResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    require_idempotency(&state, &headers).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid)
    })?;

    let challenge_b64 = consume_and_validate_challenge(
        &state,
        &rid,
        "webauthn:register",
        payload.challenge_id,
        payload.user_id,
    )
    .await?;

    let public_key_b64 = payload.public_key_b64.as_ref().ok_or_else(|| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;
    let public_key = URL_SAFE_NO_PAD
        .decode(public_key_b64.as_bytes())
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;
    let signature = URL_SAFE_NO_PAD
        .decode(payload.signature_b64.as_bytes())
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;
    let auth_data = URL_SAFE_NO_PAD
        .decode(payload.authenticator_data_b64.as_bytes())
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;
    let client_data = URL_SAFE_NO_PAD
        .decode(payload.client_data_json_b64.as_bytes())
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;

    let digest = challenge_digest(&challenge_b64, &auth_data, &client_data);
    verify_ed25519(&public_key, &digest, &signature).map_err(|_| {
        ApiError::app_with_request_id(
            transfer_legacy_shared_types::AppError::SignatureInvalid,
            &rid,
        )
    })?;

    let credential = serde_json::json!({
        "credential_id": payload.credential_id,
        "public_key_b64": URL_SAFE_NO_PAD.encode(public_key),
        "sign_count": 0,
    });

    upsert_webauthn_factor(
        &state.db,
        payload.user_id,
        credential,
        CURRENT_CRYPTO_VERSION.as_str().to_string(),
        CURRENT_SCHEMA_VERSION,
    )
    .await
    .map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    Ok(success(&rid, WebAuthnFinishResponse { status: "ok" }))
}

pub async fn authenticate_start(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<WebAuthnStartRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<WebAuthnStartResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    require_idempotency(&state, &headers).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid)
    })?;

    let challenge_id = Uuid::new_v4();
    let challenge_bytes = Uuid::new_v4().as_bytes().to_vec();
    let challenge_b64 = URL_SAFE_NO_PAD.encode(challenge_bytes);
    let key = format!("webauthn:auth:{}", challenge_id);
    let value = serde_json::json!({
        "user_id": payload.user_id,
        "challenge_b64": challenge_b64,
    })
    .to_string();

    let mut conn = state.redis_conn.clone();
    let _: () = conn.set_ex(key, value, 300).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    Ok(success(
        &rid,
        WebAuthnStartResponse {
            challenge_id,
            challenge_b64,
        },
    ))
}

pub async fn authenticate_finish(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<WebAuthnFinishRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<WebAuthnFinishResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    require_idempotency(&state, &headers).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid)
    })?;

    let challenge_b64 = consume_and_validate_challenge(
        &state,
        &rid,
        "webauthn:auth",
        payload.challenge_id,
        payload.user_id,
    )
    .await?;

    let credential =
        fetch_webauthn_credential(&state.db, payload.user_id, payload.credential_id.as_str())
            .await
            .map_err(|_| {
                ApiError::app_with_request_id(
                    transfer_legacy_shared_types::AppError::NotFound,
                    &rid,
                )
            })?;
    let stored_key_b64 = credential
        .get("public_key_b64")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;
    let public_key = URL_SAFE_NO_PAD
        .decode(stored_key_b64.as_bytes())
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    let signature = URL_SAFE_NO_PAD
        .decode(payload.signature_b64.as_bytes())
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;
    let auth_data = URL_SAFE_NO_PAD
        .decode(payload.authenticator_data_b64.as_bytes())
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;
    let client_data = URL_SAFE_NO_PAD
        .decode(payload.client_data_json_b64.as_bytes())
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;

    let digest = challenge_digest(&challenge_b64, &auth_data, &client_data);
    verify_ed25519(&public_key, &digest, &signature).map_err(|_| {
        ApiError::app_with_request_id(
            transfer_legacy_shared_types::AppError::SignatureInvalid,
            &rid,
        )
    })?;

    Ok(success(&rid, WebAuthnFinishResponse { status: "ok" }))
}

async fn consume_and_validate_challenge(
    state: &AppState,
    rid: &str,
    scope: &str,
    challenge_id: Uuid,
    user_id: Uuid,
) -> Result<String, ApiError> {
    let key = format!("{}:{}", scope, challenge_id);
    let mut conn = state.redis_conn.clone();
    let raw: Option<String> = conn.get(key.as_str()).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, rid)
    })?;
    let _: () = conn.del(key).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, rid)
    })?;

    let payload = raw.ok_or_else(|| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, rid)
    })?;
    let value: serde_json::Value = serde_json::from_str(payload.as_str()).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, rid)
    })?;

    let challenge_user_id = value
        .get("user_id")
        .and_then(|item| item.as_str())
        .ok_or_else(|| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, rid)
        })?;
    if challenge_user_id != user_id.to_string() {
        return Err(ApiError::app_with_request_id(
            transfer_legacy_shared_types::AppError::Forbidden,
            rid,
        ));
    }

    value
        .get("challenge_b64")
        .and_then(|item| item.as_str())
        .map(|item| item.to_string())
        .ok_or_else(|| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, rid)
        })
}

fn challenge_digest(challenge_b64: &str, auth_data: &[u8], client_data: &[u8]) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.extend_from_slice(challenge_b64.as_bytes());
    msg.extend_from_slice(auth_data);
    msg.extend_from_slice(client_data);
    sha256(&msg)
}
