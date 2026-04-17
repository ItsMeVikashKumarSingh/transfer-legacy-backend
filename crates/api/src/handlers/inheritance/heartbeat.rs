use axum::extract::{Extension, State};
use axum::{http::HeaderMap, Json};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::devices::fetch_device_pubkey;
use crate::db::queries::inheritance::{
    fetch_policy, insert_heartbeat_tx, update_policy_heartbeat_tx,
};
use crate::errors::ApiError;
use crate::middleware::aead_transport::{wrap_response, AeadJson, AeadResponse};
use crate::middleware::rate_limit::require_idempotency;
use crate::services::audit::{append_event, ip_hash_from_headers};
use crate::state::AppState;
use transfer_legacy_crypto_core::{hash::sha256, jcs::canonicalize, signatures::verify_ed25519};

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub policy_id: Uuid,
    pub device_id: Uuid,
    pub ts: i64,
    pub device_sig: String,
}

#[derive(Debug, Serialize)]
pub struct HeartbeatResponse {
    pub policy_id: Uuid,
    pub pending_at: chrono::DateTime<chrono::Utc>,
    pub grace_deadline: chrono::DateTime<chrono::Utc>,
    pub status: String,
}

pub async fn heartbeat(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<HeartbeatRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    require_idempotency(&state, &headers).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid)
    })?;

    let policy = fetch_policy(&state.db, payload.policy_id)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid)
        })?;

    let pubkey = fetch_device_pubkey(&state.db, policy.owner_id, payload.device_id)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(
                transfer_legacy_shared_types::AppError::Unauthorized,
                &rid,
            )
        })?;

    let canonical = canonicalize(&serde_json::json!({
        "policy_id": payload.policy_id,
        "ts": payload.ts,
        "device_id": payload.device_id,
    }))
    .map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;
    let digest = sha256(&canonical);
    let sig = URL_SAFE_NO_PAD.decode(payload.device_sig).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;

    verify_ed25519(&pubkey, &digest, &sig).map_err(|_| {
        ApiError::app_with_request_id(
            transfer_legacy_shared_types::AppError::SignatureInvalid,
            &rid,
        )
    })?;

    let ts = chrono::DateTime::from_timestamp(payload.ts, 0).ok_or_else(|| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;

    let pending_at = match policy.cadence.as_str() {
        "1w" => ts + Duration::days(7),
        "15d" => ts + Duration::days(15),
        "1m" => ts + Duration::days(30),
        "3m" => ts + Duration::days(90),
        _ => ts + Duration::days(30),
    };
    let grace_deadline = match policy.cadence.as_str() {
        "1w" => pending_at + Duration::days(28),
        "15d" => pending_at + Duration::days(45),
        "1m" => pending_at + Duration::days(90),
        "3m" => pending_at + Duration::days(90),
        _ => pending_at + Duration::days(90),
    };

    let new_status = if policy.status == "pending" || policy.status == "investigating" {
        "active"
    } else {
        policy.status.as_str()
    };

    let mut tx = state.db.begin().await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    insert_heartbeat_tx(
        &mut tx,
        payload.policy_id,
        payload.device_id,
        sig.clone(),
        ts,
    )
    .await
    .map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    update_policy_heartbeat_tx(
        &mut tx,
        payload.policy_id,
        ts,
        pending_at,
        grace_deadline,
        new_status,
    )
    .await
    .map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let heartbeat_payload = serde_json::json!({
        "policy_id": payload.policy_id,
        "device_id": payload.device_id,
        "ts": payload.ts,
        "status": new_status,
    });
    let ip_hash = ip_hash_from_headers(&headers);
    append_event(
        &mut tx,
        payload.policy_id,
        "heartbeat_received",
        &heartbeat_payload,
        Some(policy.owner_id),
        ip_hash,
    )
    .await
    .map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    tx.commit().await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let envelope = crate::errors::SuccessEnvelope {
        data: HeartbeatResponse {
            policy_id: payload.policy_id,
            pending_at,
            grace_deadline,
            status: new_status.to_string(),
        },
        request_id: rid,
    };
    let config = state.config().await;
    let aead = wrap_response(&config, &headers, &envelope)?;
    Ok(Json(aead))
}
