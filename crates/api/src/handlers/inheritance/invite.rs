use axum::extract::{Extension, Path, State};
use axum::{Json, http::HeaderMap};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::notify::insert_invite_tx;
use crate::db::queries::stepup::{fetch_stepup_challenge_tx, consume_stepup_challenge_tx};
use crate::errors::ApiError;
use crate::middleware::aead_transport::{AeadJson, AeadResponse, wrap_response};
use crate::middleware::rate_limit::require_idempotency;
use crate::notifications::brevo::send_invite_email;
use crate::services::audit::{append_event, ip_hash_from_headers};
use crate::services::hmac::compute_hmac;
use crate::state::AppState;
use crate::db::queries::inheritance::fetch_policy;

#[derive(Debug, Deserialize)]
pub struct InviteRequest {
    pub email: String,
    pub role: String,
    pub stepup_challenge_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct InviteResponse {
    pub invite_id: Uuid,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub async fn create_invite(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Path(policy_id): Path<Uuid>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<InviteRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;

    if payload.role != "beneficiary" && payload.role != "approver" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id));
    }

    let invite_id = Uuid::new_v4();
    let expires_at = Utc::now() + Duration::days(7);

    let token_data = format!("{}|{}|{}", invite_id, payload.email, expires_at.timestamp());
    let hmac = compute_hmac(state.config.server_hmac_secret.as_bytes(), token_data.as_bytes())
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;
    let claim_token = URL_SAFE_NO_PAD.encode(&hmac);

    let mut tx = state.db.begin().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let policy = fetch_policy(&state.db, policy_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &request_id))?;

    let challenge = fetch_stepup_challenge_tx(&mut tx, payload.stepup_challenge_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &request_id))?;
    if challenge.consumed_at.is_some()
        || challenge.expires_at < Utc::now()
        || challenge.user_id != policy.owner_id
        || challenge.action != "invite"
    {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &request_id));
    }
    consume_stepup_challenge_tx(&mut tx, payload.stepup_challenge_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    insert_invite_tx(&mut tx, invite_id, policy_id, &payload.email, &payload.role, hmac.clone(), expires_at)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    if let Err(e) = send_invite_email(
        &state.config,
        &payload.email,
        &invite_id.to_string(),
        &claim_token,
        &expires_at.to_rfc3339(),
    )
    .await {
        tracing::error!(request_id = %crate::middleware::request_id::request_id_string(&request_id), error = %e, "failed to send invite email");
        // We continue because the invite is already in the DB, but ideally we'd want a retry or notice to the user.
    }

    let invite_payload = serde_json::json!({
        "invite_id": invite_id,
        "policy_id": policy_id,
        "email": payload.email,
        "role": payload.role,
        "expires_at": expires_at,
    });

    let ip_hash = ip_hash_from_headers(&headers);
    append_event(&mut tx, policy_id, "invite_created", &invite_payload, None, ip_hash)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;
    tx.commit().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: InviteResponse { invite_id, expires_at },
        request_id: crate::middleware::request_id::request_id_string(&request_id),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}
