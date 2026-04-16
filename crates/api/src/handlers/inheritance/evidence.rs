use axum::extract::{Extension, State};
use axum::http::HeaderMap;
use axum::Json;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::claims::fetch_claim_for_update_tx;
use crate::db::queries::evidence::{fetch_audit_event_hashes, fetch_claim_attachments, fetch_release_record};
use crate::db::queries::inheritance::fetch_policy_for_update_tx;
use crate::errors::ApiError;
use crate::middleware::aead_transport::{AeadJson, AeadResponse, wrap_response};
use crate::middleware::rate_limit::require_idempotency;
use crate::services::{audit, openbao};
use crate::state::AppState;
use transfer_legacy_crypto_core::{hash::sha256, jcs::canonicalize};

#[derive(Debug, Deserialize)]
pub struct EvidencePackageRequest {
    pub policy_id: Uuid,
    pub claim_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct EvidencePackageResponse {
    pub evidence: serde_json::Value,
    pub signature: String,
}

pub async fn create_evidence_package(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<EvidencePackageRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid))?;

    let mut tx = state.db.begin().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let claim = fetch_claim_for_update_tx(&mut tx, payload.claim_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid))?;
    if claim.policy_id != payload.policy_id {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Forbidden, &rid));
    }

    let policy = fetch_policy_for_update_tx(&mut tx, payload.policy_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid))?;
    if policy.status != "release_ready" && policy.status != "released" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Forbidden, &rid));
    }

    tx.commit().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let audit_hashes = fetch_audit_event_hashes(&state.db, payload.policy_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;
    let audit_hashes_b64: Vec<String> = audit_hashes
        .iter()
        .map(|h| URL_SAFE_NO_PAD.encode(h))
        .collect();

    let attachments = fetch_claim_attachments(&state.db, payload.claim_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;
    let attachments_json: Vec<serde_json::Value> = attachments
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "attachment_id": row.0,
                "object_key": row.1,
                "sha256": row.2.as_ref().map(|h| URL_SAFE_NO_PAD.encode(h)),
                "size_bytes": row.3,
                "mime_type": row.4,
            })
        })
        .collect();

    let release_record = fetch_release_record(&state.db, payload.policy_id, payload.claim_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let evidence = serde_json::json!({
        "policy_id": payload.policy_id,
        "claim_id": payload.claim_id,
        "generated_at": Utc::now(),
        "audit_event_hashes": audit_hashes_b64,
        "attachments": attachments_json,
        "release_record": release_record.as_ref().map(|r| serde_json::json!({
            "release_id": r.0,
            "payload_hash": URL_SAFE_NO_PAD.encode(&r.1),
            "signature": r.2,
            "schema_version": r.3,
            "crypto_version": r.4,
        })),
    });

    let evidence_bytes = canonicalize(&evidence)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid))?;
    let digest = sha256(&evidence_bytes);
    let signature = openbao::sign_digest(&config, "tl-signing", &digest)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let mut tx = state.db.begin().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;
    let audit_payload = serde_json::json!({
        "policy_id": payload.policy_id,
        "claim_id": payload.claim_id,
        "signature": signature,
    });
    let ip_hash = audit::ip_hash_from_headers(&headers);
    audit::append_event(&mut tx, payload.policy_id, "evidence_package_created", &audit_payload, None, ip_hash)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;
    tx.commit().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: EvidencePackageResponse { evidence, signature },
        request_id: rid,
    };
    let aead = wrap_response(&config, &headers, &envelope)?;
    Ok(Json(aead))
}
