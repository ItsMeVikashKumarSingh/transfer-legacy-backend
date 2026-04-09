use axum::extract::{Extension, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::audit::fetch_audit_chain;
use crate::errors::ApiError;
use crate::middleware::aead_transport::{AeadResponse, wrap_response};
use crate::state::AppState;
use transfer_legacy_crypto_core::{hash::sha256, jcs::canonicalize};

#[derive(Debug, Deserialize)]
pub struct AuditChainQuery {
    pub policy_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct AuditChainEvent {
    pub event_id: Uuid,
    pub event_type: String,
    pub event_hash_b64: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct AuditChainResponse {
    pub policy_id: Uuid,
    pub valid: bool,
    pub invalid_at: Option<usize>,
    pub events: Vec<AuditChainEvent>,
}

pub async fn audit_chain(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Query(query): Query<AuditChainQuery>,
) -> Result<Json<AeadResponse>, ApiError> {
    let events = fetch_audit_chain(&state.db, query.policy_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let mut valid = true;
    let mut invalid_at = None;
    let mut prev: Option<Vec<u8>> = None;

    for (idx, event) in events.iter().enumerate() {
        if event.prev_hash != prev {
            valid = false;
            invalid_at = Some(idx);
            break;
        }

        let event_hash_payload = serde_json::json!({
            "event_id": event.event_id,
            "payload_hash": URL_SAFE_NO_PAD.encode(&event.payload_hash),
            "prev_hash": event.prev_hash.as_ref().map(|h| URL_SAFE_NO_PAD.encode(h)),
        });
        let event_hash_bytes = canonicalize(&event_hash_payload)
            .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;
        let computed = sha256(&event_hash_bytes);
        if computed != event.event_hash {
            valid = false;
            invalid_at = Some(idx);
            break;
        }
        prev = Some(event.event_hash.clone());
    }

    let response_events = events
        .into_iter()
        .map(|event| AuditChainEvent {
            event_id: event.event_id,
            event_type: event.event_type,
            event_hash_b64: URL_SAFE_NO_PAD.encode(&event.event_hash),
            created_at: event.created_at,
        })
        .collect();

    let envelope = crate::errors::SuccessEnvelope {
        data: AuditChainResponse {
            policy_id: query.policy_id,
            valid,
            invalid_at,
            events: response_events,
        },
        request_id: crate::middleware::request_id::request_id_string(&request_id),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}
