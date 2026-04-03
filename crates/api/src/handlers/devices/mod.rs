use axum::extract::{Extension, Path, State};
use axum::{Json, http::HeaderMap};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::{success, ApiError};
use crate::middleware::aead_transport::{AeadJson, AeadResponse, wrap_response};
use crate::middleware::rate_limit::require_idempotency;
use crate::state::AppState;
use crate::db::queries::devices::{count_devices, insert_device, list_devices, revoke_device};
use transfer_legacy_crypto_core::{hash::sha256, jcs::canonicalize, signatures::verify_ed25519};
use transfer_legacy_shared_types::{CURRENT_CRYPTO_VERSION, CURRENT_SCHEMA_VERSION};

#[derive(Debug, Deserialize)]
pub struct DeviceRegisterRequest {
    pub device_id: Uuid,
    pub user_id: Uuid,
    pub ts: i64,
    pub device_sig: String,
    pub ed25519_pubkey: String,
    pub device_meta: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct DeviceRegisterResponse {
    pub device_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct DeviceListRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct DeviceListItem {
    pub device_id: Uuid,
    pub ed25519_pubkey: String,
    pub device_meta: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DeviceListResponse {
    pub devices: Vec<DeviceListItem>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceRevokeRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct DeviceRevokeResponse {
    pub status: &'static str,
}

pub async fn register(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<DeviceRegisterRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;
    let count = count_devices(&state.db, payload.user_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;
    if count >= 10 {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id));
    }

    let ed25519_pubkey = URL_SAFE_NO_PAD.decode(payload.ed25519_pubkey)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id))?;
    let sig = URL_SAFE_NO_PAD.decode(payload.device_sig)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id))?;

    let canonical = canonicalize(&serde_json::json!({
        "device_id": payload.device_id,
        "user_id": payload.user_id,
        "ts": payload.ts,
    }))
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id))?;
    let digest = sha256(&canonical);

    verify_ed25519(&ed25519_pubkey, &digest, &sig)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::SignatureInvalid, &request_id))?;

    insert_device(
        &state.db,
        payload.device_id,
        payload.user_id,
        ed25519_pubkey,
        payload.device_meta,
        CURRENT_CRYPTO_VERSION.as_str().to_string(),
        CURRENT_SCHEMA_VERSION,
    )
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: DeviceRegisterResponse { device_id: payload.device_id },
        request_id: request_id.to_string(),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}

pub async fn list(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(payload): Json<DeviceListRequest>,
) -> Result<Json<crate::errors::SuccessEnvelope<DeviceListResponse>>, ApiError> {
    let devices = list_devices(&state.db, payload.user_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let items = devices
        .into_iter()
        .map(|d| DeviceListItem {
            device_id: d.device_id,
            ed25519_pubkey: URL_SAFE_NO_PAD.encode(d.ed25519_pubkey),
            device_meta: d.device_meta,
            created_at: d.created_at,
            last_seen_at: d.last_seen_at,
        })
        .collect::<Vec<_>>();

    Ok(success(&request_id, DeviceListResponse { devices: items }))
}

pub async fn revoke(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Path(device_id): Path<Uuid>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<DeviceRevokeRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;
    revoke_device(&state.db, payload.user_id, device_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: DeviceRevokeResponse { status: "ok" },
        request_id: request_id.to_string(),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}
