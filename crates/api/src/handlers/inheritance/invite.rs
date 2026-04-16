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
use transfer_legacy_crypto_core::aead::decrypt;
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
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid))?;

    if payload.role != "beneficiary" && payload.role != "approver" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid));
    }

    let invite_id = Uuid::new_v4();
    let expires_at = Utc::now() + Duration::days(7);

    let token_data = format!("{}|{}|{}", invite_id, payload.email, expires_at.timestamp());
    let hmac = compute_hmac(config.server_hmac_secret.as_bytes(), token_data.as_bytes())
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;
    let claim_token = URL_SAFE_NO_PAD.encode(&hmac);

    let mut tx = state.db.begin().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let policy = fetch_policy(&state.db, policy_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid))?;

    let challenge = fetch_stepup_challenge_tx(&mut tx, payload.stepup_challenge_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid))?;
    if challenge.consumed_at.is_some()
        || challenge.expires_at < Utc::now()
        || challenge.user_id != policy.owner_id
        || challenge.action != "invite"
    {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid));
    }
    consume_stepup_challenge_tx(&mut tx, payload.stepup_challenge_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    insert_invite_tx(&mut tx, invite_id, policy_id, &payload.email, &payload.role, hmac.clone(), expires_at)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let invite_url = format!("{}/invite/claim?invite_id={}&token={}", config.app_url, invite_id, claim_token);

    // Decrypt owner name if available
    let owner_name = if let Some(enc_name) = policy.enc_owner_name {
        if enc_name.len() >= 24 {
            let (nonce, ciphertext) = enc_name.split_at(24);
            let key = URL_SAFE_NO_PAD.decode(&config.server_aead_key_b64).unwrap_or_default();
            let aad = policy.owner_id.as_bytes();
            decrypt(&key, nonce, ciphertext, aad)
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_else(|_| "A Policy Owner".to_string())
        } else {
            "A Policy Owner".to_string()
        }
    } else {
        "A Policy Owner".to_string()
    };

    let policy_name = policy.label.unwrap_or_else(|| "Legacy Plan".to_string());

    let template = crate::notifications::brevo::NotificationTemplate::Invite {
        owner_name,
        policy_name,
        invite_url,
        expires_at: expires_at.to_rfc3339(),
        invite_id: invite_id.to_string(),
        claim_token: claim_token.clone(),
    };

    if let Err(e) = state.notify(policy.owner_id, &payload.email, template).await {
        tracing::error!(request_id = %rid, error = %e, "failed to send invite email");
    }

    let invite_payload = serde_json::json!({
        "invite_id": invite_id,
        "policy_id": policy_id,
        "email": payload.email,
        "role": payload.role,
        "expires_at": expires_at,
    });

    let ip_hash = ip_hash_from_headers(&headers);
    append_event(&mut tx, policy_id, "invite_created", &invite_payload, Some(policy.owner_id), ip_hash)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;
    tx.commit().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: InviteResponse { invite_id, expires_at },
        request_id: rid,
    };
    let aead = wrap_response(&config, &headers, &envelope)?;
    Ok(Json(aead))
}
