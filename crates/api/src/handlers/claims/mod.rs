use axum::extract::{Extension, State};
use axum::http::HeaderMap;
use axum::Json;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::claims::{
    confirm_attachment_tx, fetch_attachment_policy_tx, fetch_claim_for_update_tx, fetch_claim_policy,
    fetch_policy_approvers, insert_attestation_tx, insert_attachment_tx, insert_claim_tx,
    insert_release_record_tx, update_claim_status_tx,
};
use crate::db::queries::inheritance::fetch_policy_for_update_tx;
use crate::errors::ApiError;
use crate::middleware::aead_transport::{AeadJson, AeadResponse, wrap_response};
use crate::middleware::rate_limit::require_idempotency;
use crate::services::{audit, b2, openbao};
use crate::state::AppState;
use transfer_legacy_crypto_core::{hash::sha256, jcs::canonicalize, signatures::verify_ed25519};

#[derive(Debug, Deserialize)]
pub struct ClaimInitiateRequest {
    pub policy_id: Uuid,
    pub claimant_person_id: Uuid,
    pub claim_type: String,
}

#[derive(Debug, Serialize)]
pub struct ClaimInitiateResponse {
    pub claim_id: Uuid,
    pub confirmation_deadline: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn initiate_claim(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<ClaimInitiateRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;

    if payload.claim_type != "type_a" && payload.claim_type != "type_b" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id));
    }

    let mut tx = state.db.begin().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let policy = fetch_policy_for_update_tx(&mut tx, payload.policy_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &request_id))?;

    if policy.status != "investigating" && policy.status != "release_ready" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Forbidden, &request_id));
    }

    let (status, confirmation_deadline) = if payload.claim_type == "type_a" {
        ("pending_confirmation", Some(Utc::now() + Duration::days(7)))
    } else {
        ("confirmed", None)
    };

    let claim_id = insert_claim_tx(
        &mut tx,
        payload.policy_id,
        payload.claimant_person_id,
        &payload.claim_type,
        status,
        confirmation_deadline,
    )
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    if status == "confirmed" {
        sqlx::query("UPDATE inheritance.policies SET status = 'release_ready', updated_at = now() WHERE policy_id = $1 AND status = 'investigating'")
            .bind(payload.policy_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;
    }

    let audit_payload = serde_json::json!({
        "claim_id": claim_id,
        "policy_id": payload.policy_id,
        "claimant_person_id": payload.claimant_person_id,
        "claim_type": payload.claim_type,
        "status": status,
        "confirmation_deadline": confirmation_deadline,
    });
    audit::append_event(&mut tx, payload.policy_id, "claim_initiated", &audit_payload, Some(policy.owner_id), None)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    tx.commit().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: ClaimInitiateResponse { claim_id, confirmation_deadline },
        request_id: request_id.to_string(),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}

#[derive(Debug, Deserialize)]
pub struct ClaimConfirmRequest {
    pub claim_id: Uuid,
    pub claimant_person_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ClaimConfirmResponse {
    pub status: &'static str,
}

pub async fn confirm_claim(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<ClaimConfirmRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;

    let mut tx = state.db.begin().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let claim = fetch_claim_for_update_tx(&mut tx, payload.claim_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &request_id))?;

    if claim.claimant_person_id != payload.claimant_person_id {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Forbidden, &request_id));
    }

    if claim.claim_type != "type_a" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id));
    }

    if claim.status != "pending_confirmation" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id));
    }

    if let Some(deadline) = claim.confirmation_deadline {
        if deadline < Utc::now() {
            return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &request_id));
        }
    }

    update_claim_status_tx(&mut tx, payload.claim_id, "confirmed", Some(Utc::now()))
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    sqlx::query("UPDATE inheritance.policies SET status = 'release_ready', updated_at = now() WHERE policy_id = $1 AND status = 'investigating'")
        .bind(claim.policy_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let audit_payload = serde_json::json!({
        "claim_id": payload.claim_id,
        "policy_id": claim.policy_id,
        "status": "confirmed",
    });
    audit::append_event(&mut tx, claim.policy_id, "claim_confirmed", &audit_payload, None, None)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    tx.commit().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: ClaimConfirmResponse { status: "ok" },
        request_id: request_id.to_string(),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}

#[derive(Debug, Deserialize)]
pub struct PresignAttachmentRequest {
    pub claim_id: Uuid,
    pub content_type: String,
}

#[derive(Debug, Serialize)]
pub struct PresignAttachmentResponse {
    pub attachment_id: Uuid,
    pub upload_url: String,
    pub object_key: String,
}

pub async fn presign_attachment(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<PresignAttachmentRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;

    let mut tx = state.db.begin().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let attachment_id = Uuid::new_v4();
    let object_key = format!("claims/{}/{}", payload.claim_id, attachment_id);
    insert_attachment_tx(&mut tx, attachment_id, payload.claim_id, &object_key)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    tx.commit().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let upload_url = b2::presign_put(&state.config, &object_key, &payload.content_type, 900)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: PresignAttachmentResponse { attachment_id, upload_url, object_key },
        request_id: request_id.to_string(),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}

#[derive(Debug, Deserialize)]
pub struct ConfirmAttachmentRequest {
    pub attachment_id: Uuid,
    pub sha256_b64: String,
    pub size_bytes: i64,
    pub mime_type: String,
}

#[derive(Debug, Serialize)]
pub struct ConfirmAttachmentResponse {
    pub status: &'static str,
}

pub async fn confirm_attachment(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<ConfirmAttachmentRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;

    let hash = URL_SAFE_NO_PAD
        .decode(payload.sha256_b64)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id))?;

    let mut tx = state.db.begin().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let (claim_id, policy_id) = fetch_attachment_policy_tx(&mut tx, payload.attachment_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &request_id))?;

    confirm_attachment_tx(&mut tx, payload.attachment_id, hash, payload.size_bytes, &payload.mime_type)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let audit_payload = serde_json::json!({
        "attachment_id": payload.attachment_id,
        "claim_id": claim_id,
        "policy_id": policy_id,
    });
    audit::append_event(&mut tx, policy_id, "claim_attachment_confirmed", &audit_payload, None, None)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    tx.commit().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: ConfirmAttachmentResponse { status: "ok" },
        request_id: request_id.to_string(),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}

#[derive(Debug, Deserialize)]
pub struct AttestationRequest {
    pub policy_id: Uuid,
    pub claim_id: Uuid,
    pub approver_person_id: Uuid,
    pub statement: serde_json::Value,
    pub signature_b64: String,
    pub public_key_b64: String,
    pub signature_type: String,
}

#[derive(Debug, Serialize)]
pub struct AttestationResponse {
    pub attestation_id: Uuid,
}

pub async fn submit_attestation(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<AttestationRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;

    if payload.signature_type != "ed25519" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id));
    }

    let approvers = fetch_policy_approvers(&state.db, payload.policy_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &request_id))?;

    let claim_policy_id = fetch_claim_policy(&state.db, payload.claim_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &request_id))?;
    if claim_policy_id != payload.policy_id {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Forbidden, &request_id));
    }

    let allowed = approvers.as_array().map(|arr| {
        arr.iter().any(|v| {
            v.get("person_id")
                .and_then(|p| p.as_str())
                .map(|s| s == payload.approver_person_id.to_string())
                .unwrap_or(false)
        })
    }).unwrap_or(false);

    if !allowed {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Forbidden, &request_id));
    }

    let statement_bytes = canonicalize(&payload.statement)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id))?;
    let digest = sha256(&statement_bytes);
    let signature = URL_SAFE_NO_PAD
        .decode(payload.signature_b64)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id))?;
    let public_key = URL_SAFE_NO_PAD
        .decode(payload.public_key_b64)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id))?;

    verify_ed25519(&public_key, &digest, &signature)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::SignatureInvalid, &request_id))?;

    let mut tx = state.db.begin().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let attestation_id = insert_attestation_tx(
        &mut tx,
        payload.policy_id,
        payload.claim_id,
        payload.approver_person_id,
        payload.statement,
        signature,
        &payload.signature_type,
    )
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let audit_payload = serde_json::json!({
        "attestation_id": attestation_id,
        "policy_id": payload.policy_id,
        "claim_id": payload.claim_id,
        "approver_person_id": payload.approver_person_id,
    });
    audit::append_event(&mut tx, payload.policy_id, "attestation_submitted", &audit_payload, None, None)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    tx.commit().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: AttestationResponse { attestation_id },
        request_id: request_id.to_string(),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}

#[derive(Debug, Deserialize)]
pub struct ReleaseRecordRequest {
    pub policy_id: Uuid,
    pub claim_id: Uuid,
    pub payload: serde_json::Value,
    pub schema_version: i32,
    pub crypto_version: String,
}

#[derive(Debug, Serialize)]
pub struct ReleaseRecordResponse {
    pub release_id: Uuid,
    pub signature: String,
}

pub async fn create_release_record(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<ReleaseRecordRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;

    let payload_bytes = canonicalize(&payload.payload)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id))?;
    let payload_hash = sha256(&payload_bytes);
    let signature = openbao::sign_digest(&state.config, "tl-signing", &payload_hash)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let mut tx = state.db.begin().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let release_id = insert_release_record_tx(
        &mut tx,
        payload.policy_id,
        payload.claim_id,
        payload_hash,
        &signature,
        payload.schema_version,
        &payload.crypto_version,
    )
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let audit_payload = serde_json::json!({
        "release_id": release_id,
        "policy_id": payload.policy_id,
        "claim_id": payload.claim_id,
    });
    audit::append_event(&mut tx, payload.policy_id, "release_record_created", &audit_payload, None, None)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    tx.commit().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: ReleaseRecordResponse { release_id, signature },
        request_id: request_id.to_string(),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}
