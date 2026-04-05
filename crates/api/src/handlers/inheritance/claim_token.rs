use axum::extract::{Extension, State};
use axum::{Json, http::HeaderMap};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::inheritance::{fetch_policy_for_update_tx, update_policy_participants};
use crate::db::queries::notify::{fetch_invite_for_update, mark_invite_used};
use crate::errors::ApiError;
use crate::middleware::aead_transport::{AeadJson, AeadResponse, wrap_response};
use crate::middleware::rate_limit::require_idempotency;
use crate::services::audit::{append_event, ip_hash_from_headers};
use crate::services::hmac::verify_hmac;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ClaimTokenConsumeRequest {
    pub invite_id: Uuid,
    pub claim_token: String,
    pub person_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ClaimTokenConsumeResponse {
    pub status: &'static str,
}

pub async fn consume_claim_token(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<ClaimTokenConsumeRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;

    let mut tx = state.db.begin().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let _ = sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(payload.invite_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let invite = fetch_invite_for_update(&mut tx, payload.invite_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &request_id))?;

    if invite.used || invite.expires_at < Utc::now() {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &request_id));
    }

    let token_bytes = URL_SAFE_NO_PAD.decode(payload.claim_token)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id))?;
    let data = format!("{}|{}|{}", invite.invite_id, invite.email, invite.expires_at.timestamp());
    verify_hmac(state.config.server_hmac_secret.as_bytes(), data.as_bytes(), &token_bytes)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &request_id))?;
    if token_bytes != invite.claim_token_hmac {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &request_id));
    }

    let policy = fetch_policy_for_update_tx(&mut tx, invite.policy_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let mut beneficiaries_json = policy.beneficiaries.as_array().cloned().unwrap_or_default();
    let mut approvers_json = policy.approvers.as_array().cloned().unwrap_or_default();

    let entry = serde_json::json!({
        "person_id": payload.person_id,
        "email": invite.email,
        "invite_id": invite.invite_id,
        "accepted_at": Utc::now(),
    });

    if invite.role == "beneficiary" {
        beneficiaries_json.push(entry);
    } else if invite.role == "approver" {
        approvers_json.push(entry);
    } else {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id));
    }

    update_policy_participants(&mut tx, invite.policy_id, serde_json::Value::Array(beneficiaries_json), serde_json::Value::Array(approvers_json))
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    mark_invite_used(&mut tx, invite.invite_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let consume_payload = serde_json::json!({
        "invite_id": invite.invite_id,
        "policy_id": invite.policy_id,
        "person_id": payload.person_id,
    });
    let ip_hash = ip_hash_from_headers(&headers);
    append_event(&mut tx, invite.policy_id, "invite_consumed", &consume_payload, None, ip_hash)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    tx.commit().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: ClaimTokenConsumeResponse { status: "ok" },
        request_id: request_id.to_string(),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}
