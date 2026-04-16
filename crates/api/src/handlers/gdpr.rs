use axum::extract::{Extension, State};
use axum::http::HeaderMap;
use axum::Json;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::ApiError;
use crate::middleware::aead_transport::{AeadJson, AeadResponse, wrap_response};
use crate::middleware::rate_limit::require_idempotency;
use crate::state::AppState;
use transfer_legacy_crypto_core::aead;

#[derive(Debug, Deserialize)]
pub struct GdprExportRequest {
    pub user_id: Uuid,
    pub person_id: Uuid,
    pub export_key_b64: String,
}

#[derive(Debug, Serialize)]
pub struct GdprExportResponse {
    pub nonce_b64: String,
    pub ciphertext_b64: String,
}

pub async fn export_gdpr(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<GdprExportRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid))?;

    let key = URL_SAFE_NO_PAD
        .decode(payload.export_key_b64)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid))?;
    if key.len() != 32 {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid));
    }

    let person = sqlx::query_as::<_, (Uuid, Vec<u8>, Vec<u8>, String)>(
        "SELECT person_id, enc_legal_name, enc_email, kyc_status::text FROM auth_ext.persons WHERE person_id = $1",
    )
    .bind(payload.person_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid))?;

    let links = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT person_id, user_id FROM auth_ext.person_user_links WHERE person_id = $1",
    )
    .bind(payload.person_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let linked = links.iter().any(|row| row.1 == payload.user_id);
    if !linked {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Forbidden, &rid));
    }

    let devices = sqlx::query_as::<_, (Uuid, Vec<u8>, Option<serde_json::Value>)>(
        "SELECT device_id, ed25519_pubkey, device_meta FROM auth_ext.devices WHERE user_id = $1 AND is_deleted = false",
    )
    .bind(payload.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let opaque = sqlx::query_as::<_, (Uuid, Vec<u8>, Vec<u8>, serde_json::Value)>(
        "SELECT user_id, opaque_record, emk_blob, argon2_params FROM auth_ext.opaque_records WHERE user_id = $1 AND is_deleted = false",
    )
    .bind(payload.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let vault_items = sqlx::query_as::<_, (Uuid, Vec<u8>, Option<serde_json::Value>)>(
        "SELECT item_id, ciphertext, item_meta FROM vault.items WHERE user_id = $1 AND is_deleted = false",
    )
    .bind(payload.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let vault_shares = sqlx::query_as::<_, (Uuid, Uuid, Vec<u8>, Vec<u8>)>(
        "SELECT share_id, item_id, envelope, grant_sig FROM vault.shares WHERE owner_id = $1 AND is_deleted = false",
    )
    .bind(payload.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let policies = sqlx::query_as::<_, (Uuid, String, String, serde_json::Value, serde_json::Value, Option<serde_json::Value>, String)>(
        "SELECT policy_id, policy_type::text, cadence::text, beneficiaries, approvers, release_conditions, status::text FROM inheritance.policies WHERE owner_id = $1 AND is_deleted = false",
    )
    .bind(payload.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let claims = sqlx::query_as::<_, (Uuid, Uuid, String, String)>(
        "SELECT claim_id, policy_id, claim_type::text, status::text FROM inheritance.claims WHERE claimant_person_id = $1",
    )
    .bind(payload.person_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let attestations = sqlx::query_as::<_, (Uuid, Uuid, Uuid, serde_json::Value)>(
        "SELECT attestation_id, policy_id, claim_id, statement FROM inheritance.attestations WHERE approver_person_id = $1",
    )
    .bind(payload.person_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let export = serde_json::json!({
        "person": {
            "person_id": person.0,
            "enc_legal_name": URL_SAFE_NO_PAD.encode(person.1),
            "enc_email": URL_SAFE_NO_PAD.encode(person.2),
            "kyc_status": person.3,
        },
        "links": links.iter().map(|row| serde_json::json!({ "person_id": row.0, "user_id": row.1 })).collect::<Vec<_>>(),
        "devices": devices.iter().map(|row| serde_json::json!({
            "device_id": row.0,
            "ed25519_pubkey": URL_SAFE_NO_PAD.encode(&row.1),
            "device_meta": row.2,
        })).collect::<Vec<_>>(),
        "opaque_record": opaque.as_ref().map(|row| serde_json::json!({
            "user_id": row.0,
            "opaque_record": URL_SAFE_NO_PAD.encode(&row.1),
            "emk_blob": URL_SAFE_NO_PAD.encode(&row.2),
            "argon2_params": row.3,
        })),
        "vault_items": vault_items.iter().map(|row| serde_json::json!({
            "item_id": row.0,
            "ciphertext": URL_SAFE_NO_PAD.encode(&row.1),
            "item_meta": row.2,
        })).collect::<Vec<_>>(),
        "vault_shares": vault_shares.iter().map(|row| serde_json::json!({
            "share_id": row.0,
            "item_id": row.1,
            "envelope": URL_SAFE_NO_PAD.encode(&row.2),
            "grant_sig": URL_SAFE_NO_PAD.encode(&row.3),
        })).collect::<Vec<_>>(),
        "policies": policies.iter().map(|row| serde_json::json!({
            "policy_id": row.0,
            "policy_type": row.1,
            "cadence": row.2,
            "beneficiaries": row.3,
            "approvers": row.4,
            "release_conditions": row.5,
            "status": row.6,
        })).collect::<Vec<_>>(),
        "claims": claims.iter().map(|row| serde_json::json!({
            "claim_id": row.0,
            "policy_id": row.1,
            "claim_type": row.2,
            "status": row.3,
        })).collect::<Vec<_>>(),
        "attestations": attestations.iter().map(|row| serde_json::json!({
            "attestation_id": row.0,
            "policy_id": row.1,
            "claim_id": row.2,
            "statement": row.3,
        })).collect::<Vec<_>>(),
    });

    let plaintext = serde_json::to_vec(&export)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;
    let aad = payload.user_id.as_bytes();
    let envelope = aead::encrypt(&key, &plaintext, aad)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let response = GdprExportResponse {
        nonce_b64: URL_SAFE_NO_PAD.encode(envelope.nonce),
        ciphertext_b64: URL_SAFE_NO_PAD.encode(envelope.ciphertext),
    };

    let envelope = crate::errors::SuccessEnvelope {
        data: response,
        request_id: rid,
    };
    let config = state.config().await;
    let aead = wrap_response(&config, &headers, &envelope)?;
    Ok(Json(aead))
}

#[derive(Debug, Deserialize)]
pub struct GdprEraseRequest {
    pub user_id: Uuid,
    pub person_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct GdprEraseResponse {
    pub status: &'static str,
}

pub async fn erase_gdpr(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<GdprEraseRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid))?;

    let mut tx = state.db.begin().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let linked = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM auth_ext.person_user_links WHERE person_id = $1 AND user_id = $2",
    )
    .bind(payload.person_id)
    .bind(payload.user_id)
    .fetch_one(tx.as_mut())
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;
    if linked == 0 {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Forbidden, &rid));
    }

    sqlx::query("UPDATE vault.items SET ciphertext = ''::bytea, is_deleted = true, deleted_at = now() WHERE user_id = $1")
        .bind(payload.user_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    sqlx::query("UPDATE vault.shares SET envelope = ''::bytea, grant_sig = ''::bytea, is_deleted = true, deleted_at = now() WHERE owner_id = $1")
        .bind(payload.user_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    sqlx::query("UPDATE auth_ext.opaque_records SET opaque_record = ''::bytea, emk_blob = ''::bytea, is_deleted = true, deleted_at = now() WHERE user_id = $1")
        .bind(payload.user_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    sqlx::query("UPDATE auth_ext.devices SET ed25519_pubkey = ''::bytea, is_deleted = true, deleted_at = now() WHERE user_id = $1")
        .bind(payload.user_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    sqlx::query("UPDATE auth_ext.mfa_factors SET enabled = false, totp_secret_enc = ''::bytea, is_deleted = true, deleted_at = now() WHERE user_id = $1")
        .bind(payload.user_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    sqlx::query("UPDATE inheritance.policies SET beneficiaries = '[]'::jsonb, approvers = '[]'::jsonb, release_conditions = NULL, is_deleted = true, deleted_at = now() WHERE owner_id = $1")
        .bind(payload.user_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    sqlx::query("UPDATE inheritance.claim_attachments SET object_key = '', status = 'rejected' WHERE claim_id IN (SELECT claim_id FROM inheritance.claims WHERE claimant_person_id = $1)")
        .bind(payload.person_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    sqlx::query("UPDATE auth_ext.persons SET enc_legal_name = ''::bytea, enc_email = ''::bytea, is_deleted = true, deleted_at = now() WHERE person_id = $1")
        .bind(payload.person_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    sqlx::query("DELETE FROM auth_ext.person_user_links WHERE person_id = $1")
        .bind(payload.person_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    tx.commit().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: GdprEraseResponse { status: "ok" },
        request_id: rid,
    };
    let config = state.config().await;
    let aead = wrap_response(&config, &headers, &envelope)?;
    Ok(Json(aead))
}
