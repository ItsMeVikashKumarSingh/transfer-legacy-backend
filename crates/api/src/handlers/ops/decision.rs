use axum::extract::{Extension, Path, State};
use axum::http::HeaderMap;
use axum::Json;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::ops::{fetch_manual_review_for_update, update_manual_review_decision};
use crate::errors::ApiError;
use crate::middleware::aead_transport::{wrap_response, AeadJson, AeadResponse};
use crate::middleware::rate_limit::require_idempotency;
use crate::services::audit::{append_event, ip_hash_from_headers};
use crate::services::fraud::bump_fraud_counter;
use crate::state::AppState;
use transfer_legacy_crypto_core::{hash::sha256, jcs::canonicalize, signatures::verify_ed25519};

#[derive(Debug, Deserialize)]
pub struct ReviewDecisionRequest {
    pub decision: String,
    pub notes: serde_json::Value,
    pub operator_a_id: Uuid,
    pub operator_a_public_key_b64: String,
    pub operator_a_signature_b64: String,
    pub operator_b_id: Uuid,
    pub operator_b_public_key_b64: String,
    pub operator_b_signature_b64: String,
}

#[derive(Debug, Serialize)]
pub struct ReviewDecisionResponse {
    pub status: &'static str,
    pub review_id: Uuid,
    pub policy_id: Uuid,
}

pub async fn review_decision(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Path(review_id): Path<Uuid>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<ReviewDecisionRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    require_idempotency(&state, &headers).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid)
    })?;

    if payload.decision != "released" && payload.decision != "cancelled" {
        return Err(ApiError::app_with_request_id(
            transfer_legacy_shared_types::AppError::BadRequest,
            &rid,
        ));
    }

    if payload.operator_a_id == payload.operator_b_id {
        let _ = bump_fraud_counter(&state, "manual_review_same_operator").await;
        return Err(ApiError::app_with_request_id(
            transfer_legacy_shared_types::AppError::DualSignatureRequired,
            &rid,
        ));
    }

    let sig_payload = serde_json::json!({
        "review_id": review_id,
        "decision": payload.decision,
        "notes": payload.notes,
        "operator_a_id": payload.operator_a_id,
        "operator_b_id": payload.operator_b_id,
    });
    let canonical = canonicalize(&sig_payload).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;
    let digest = sha256(&canonical);

    let op_a_pub = URL_SAFE_NO_PAD
        .decode(payload.operator_a_public_key_b64)
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;
    let op_a_sig = URL_SAFE_NO_PAD
        .decode(payload.operator_a_signature_b64)
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;
    let op_b_pub = URL_SAFE_NO_PAD
        .decode(payload.operator_b_public_key_b64)
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;
    let op_b_sig = URL_SAFE_NO_PAD
        .decode(payload.operator_b_signature_b64)
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;

    let op_a_valid = verify_ed25519(&op_a_pub, &digest, &op_a_sig).is_ok();
    let op_b_valid = verify_ed25519(&op_b_pub, &digest, &op_b_sig).is_ok();
    if !op_a_valid || !op_b_valid {
        let _ = bump_fraud_counter(&state, "manual_review_invalid_signature").await;
        return Err(ApiError::app_with_request_id(
            transfer_legacy_shared_types::AppError::DualSignatureRequired,
            &rid,
        ));
    }

    let mut tx = state.db.begin().await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let review = fetch_manual_review_for_update(&mut tx, review_id)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid)
        })?;
    if review.status != "open" {
        return Err(ApiError::app_with_request_id(
            transfer_legacy_shared_types::AppError::Conflict,
            &rid,
        ));
    }

    let decision_notes = serde_json::json!({
        "decision": payload.decision,
        "notes": payload.notes,
        "operator_a_id": payload.operator_a_id,
        "operator_b_id": payload.operator_b_id,
    });
    update_manual_review_decision(
        &mut tx,
        review_id,
        &payload.decision,
        decision_notes.clone(),
    )
    .await
    .map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let audit_payload = serde_json::json!({
        "review_id": review_id,
        "policy_id": review.policy_id,
        "decision": payload.decision,
        "operator_a_id": payload.operator_a_id,
        "operator_b_id": payload.operator_b_id,
    });
    let ip_hash = ip_hash_from_headers(&headers);
    append_event(
        &mut tx,
        review.policy_id,
        "manual_review_decision",
        &audit_payload,
        None,
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
        data: ReviewDecisionResponse {
            status: "ok",
            review_id,
            policy_id: review.policy_id,
        },
        request_id: rid,
    };
    let config = state.config().await;
    let aead = wrap_response(&config, &headers, &envelope)?;
    Ok(Json(aead))
}
