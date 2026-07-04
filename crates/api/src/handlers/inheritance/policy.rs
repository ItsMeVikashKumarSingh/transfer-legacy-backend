use axum::extract::{Extension, State};
use axum::{http::HeaderMap, Json};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::inheritance::{
    fetch_policy_for_update_tx, insert_policy_tx, update_policy_tx, fetch_policy_by_owner,
};
use crate::db::queries::stepup::{consume_stepup_challenge_tx, fetch_stepup_challenge_tx};
use crate::errors::ApiError;
use crate::middleware::aead_transport::{wrap_response, AeadJson, AeadResponse};
use crate::middleware::rate_limit::require_idempotency;
use crate::services::audit::{append_event, ip_hash_from_headers};
use crate::state::AppState;
use transfer_legacy_shared_types::{CURRENT_CRYPTO_VERSION, CURRENT_SCHEMA_VERSION};

#[derive(Debug, Deserialize)]
pub struct PolicyUpsertRequest {
    pub owner_id: Uuid,
    pub policy_id: Option<Uuid>,
    pub policy_type: String,
    pub cadence: String,
    pub m_of_n: Option<serde_json::Value>,
    pub beneficiaries: serde_json::Value,
    pub approvers: serde_json::Value,
    pub release_conditions: Option<serde_json::Value>,
    pub label: Option<String>,
    pub stepup_challenge_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct PolicyUpsertResponse {
    pub policy_id: Uuid,
    pub pending_at: chrono::DateTime<chrono::Utc>,
    pub grace_deadline: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PolicyGetRequest {
    pub owner_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct PolicyGetResponse {
    pub policy_id: Uuid,
    pub policy_type: String,
    pub cadence: String,
    pub m_of_n: Option<serde_json::Value>,
    pub beneficiaries: serde_json::Value,
    pub approvers: serde_json::Value,
    pub status: String,
    pub last_heartbeat_at: Option<chrono::DateTime<chrono::Utc>>,
    pub pending_at: Option<chrono::DateTime<chrono::Utc>>,
    pub grace_deadline: Option<chrono::DateTime<chrono::Utc>>,
    pub label: Option<String>,
}

pub async fn get_policy_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<PolicyGetRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let policy = fetch_policy_by_owner(&state.db, payload.owner_id)
        .await
        .map_err(|e| {
            if matches!(e, sqlx::Error::RowNotFound) {
                ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid)
            } else {
                ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
            }
        })?;

    let envelope = crate::errors::SuccessEnvelope {
        data: PolicyGetResponse {
            policy_id: policy.policy_id,
            policy_type: policy.policy_type,
            cadence: policy.cadence,
            m_of_n: policy.m_of_n,
            beneficiaries: policy.beneficiaries,
            approvers: policy.approvers,
            status: policy.status,
            last_heartbeat_at: policy.last_heartbeat_at,
            pending_at: policy.pending_at,
            grace_deadline: policy.grace_deadline,
            label: policy.label,
        },
        request_id: rid,
    };
    let config = state.config().await;
    let aead = wrap_response(&config, &headers, &envelope)?;
    Ok(Json(aead))
}

pub async fn upsert_policy(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<PolicyUpsertRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    require_idempotency(&state, &headers).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid)
    })?;

    validate_m_of_n(&payload.policy_type, payload.m_of_n.as_ref()).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;

    let mut tx = state.db.begin().await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let challenge = fetch_stepup_challenge_tx(&mut tx, payload.stepup_challenge_id)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(
                transfer_legacy_shared_types::AppError::Unauthorized,
                &rid,
            )
        })?;
    if challenge.consumed_at.is_some()
        || challenge.expires_at < Utc::now()
        || challenge.user_id != payload.owner_id
        || challenge.action != "policy_upsert"
    {
        return Err(ApiError::app_with_request_id(
            transfer_legacy_shared_types::AppError::Unauthorized,
            &rid,
        ));
    }
    consume_stepup_challenge_tx(&mut tx, payload.stepup_challenge_id)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    let cadence_days = cadence_to_days(&payload.cadence).ok_or_else(|| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;

    let (policy_id, event_type, pending_at, grace_deadline) = if let Some(policy_id) =
        payload.policy_id
    {
        let existing = fetch_policy_for_update_tx(&mut tx, policy_id)
            .await
            .map_err(|_| {
                ApiError::app_with_request_id(
                    transfer_legacy_shared_types::AppError::NotFound,
                    &rid,
                )
            })?;
        if existing.owner_id != payload.owner_id {
            return Err(ApiError::app_with_request_id(
                transfer_legacy_shared_types::AppError::Forbidden,
                &rid,
            ));
        }
        let base_time = existing.last_heartbeat_at.unwrap_or_else(Utc::now);
        let pending_at = base_time + Duration::days(cadence_days);
        let grace_deadline =
            grace_deadline_from_cadence(&payload.cadence, pending_at).ok_or_else(|| {
                ApiError::app_with_request_id(
                    transfer_legacy_shared_types::AppError::BadRequest,
                    &rid,
                )
            })?;
        let update_result = update_policy_tx(
            &mut tx,
            policy_id,
            payload.owner_id,
            &payload.policy_type,
            &payload.cadence,
            payload.m_of_n.clone(),
            payload.beneficiaries.clone(),
            payload.approvers.clone(),
            payload.release_conditions.clone(),
            Some(pending_at),
            Some(grace_deadline),
            payload.label.as_deref(),
        )
        .await;
        if let Err(err) = update_result {
            if matches!(err, sqlx::Error::RowNotFound) {
                return Err(ApiError::app_with_request_id(
                    transfer_legacy_shared_types::AppError::NotFound,
                    &rid,
                ));
            }
            return Err(ApiError::app_with_request_id(
                transfer_legacy_shared_types::AppError::Internal,
                &rid,
            ));
        }
        (policy_id, "policy_updated", pending_at, grace_deadline)
    } else {
        let now = Utc::now();
        let pending_at = now + Duration::days(cadence_days);
        let grace_deadline =
            grace_deadline_from_cadence(&payload.cadence, pending_at).ok_or_else(|| {
                ApiError::app_with_request_id(
                    transfer_legacy_shared_types::AppError::BadRequest,
                    &rid,
                )
            })?;
        let policy_id = insert_policy_tx(
            &mut tx,
            payload.owner_id,
            &payload.policy_type,
            &payload.cadence,
            payload.m_of_n.clone(),
            payload.beneficiaries.clone(),
            payload.approvers.clone(),
            payload.release_conditions.clone(),
            "active",
            Some(now),
            Some(pending_at),
            Some(grace_deadline),
            CURRENT_CRYPTO_VERSION.as_str().to_string(),
            CURRENT_SCHEMA_VERSION,
            payload.label.as_deref(),
        )
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;
        (policy_id, "policy_created", pending_at, grace_deadline)
    };

    let policy_payload = serde_json::json!({
        "policy_id": policy_id,
        "owner_id": payload.owner_id,
        "policy_type": payload.policy_type,
        "cadence": payload.cadence,
        "m_of_n": payload.m_of_n,
        "beneficiaries": payload.beneficiaries,
        "approvers": payload.approvers,
        "release_conditions": payload.release_conditions,
        "pending_at": pending_at,
        "grace_deadline": grace_deadline,
    });

    let ip_hash = ip_hash_from_headers(&headers);
    append_event(
        &mut tx,
        policy_id,
        event_type,
        &policy_payload,
        Some(payload.owner_id),
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
        data: PolicyUpsertResponse {
            policy_id,
            pending_at,
            grace_deadline,
        },
        request_id: rid,
    };
    let config = state.config().await;
    let aead = wrap_response(&config, &headers, &envelope)?;
    Ok(Json(aead))
}

fn validate_m_of_n(policy_type: &str, m_of_n: Option<&serde_json::Value>) -> Result<(), ()> {
    if policy_type == "m_of_n" {
        let obj = m_of_n.and_then(|v| v.as_object()).ok_or(())?;
        let m = obj.get("m").and_then(|v| v.as_i64()).ok_or(())?;
        let n = obj.get("n").and_then(|v| v.as_i64()).ok_or(())?;
        if m < 1 || n < 1 || m > n {
            return Err(());
        }
    }
    Ok(())
}

fn cadence_to_days(cadence: &str) -> Option<i64> {
    match cadence {
        "1w" => Some(7),
        "15d" => Some(15),
        "1m" => Some(30),
        "3m" => Some(90),
        _ => None,
    }
}

fn grace_deadline_from_cadence(
    cadence: &str,
    pending_at: chrono::DateTime<chrono::Utc>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    match cadence {
        "1w" => Some(pending_at + Duration::days(28)),
        "15d" => Some(pending_at + Duration::days(45)),
        "1m" => Some(pending_at + Duration::days(90)),
        "3m" => Some(pending_at + Duration::days(90)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{cadence_to_days, grace_deadline_from_cadence};
    use chrono::{Duration, TimeZone, Utc};

    #[test]
    fn cadence_to_days_mapping() {
        assert_eq!(cadence_to_days("1w"), Some(7));
        assert_eq!(cadence_to_days("15d"), Some(15));
        assert_eq!(cadence_to_days("1m"), Some(30));
        assert_eq!(cadence_to_days("3m"), Some(90));
        assert_eq!(cadence_to_days("unknown"), None);
    }

    #[test]
    fn grace_deadline_rules_match_spec() {
        let pending = Utc.with_ymd_and_hms(2026, 4, 4, 0, 0, 0).unwrap();
        assert_eq!(
            grace_deadline_from_cadence("1w", pending),
            Some(pending + Duration::days(28))
        );
        assert_eq!(
            grace_deadline_from_cadence("15d", pending),
            Some(pending + Duration::days(45))
        );
        assert_eq!(
            grace_deadline_from_cadence("1m", pending),
            Some(pending + Duration::days(90))
        );
        assert_eq!(
            grace_deadline_from_cadence("3m", pending),
            Some(pending + Duration::days(90))
        );
    }
}
