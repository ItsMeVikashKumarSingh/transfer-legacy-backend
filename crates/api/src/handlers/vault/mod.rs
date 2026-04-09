use axum::extract::{Extension, State};
use axum::{Json, http::HeaderMap};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::ApiError;
use crate::middleware::aead_transport::{AeadJson, AeadResponse, wrap_response};
use crate::middleware::rate_limit::require_idempotency;
use crate::state::AppState;
use crate::db::queries::vault::{
    insert_item, list_items, get_item, delete_item, insert_share, list_shares, revoke_share,
};
use crate::db::queries::auth::fetch_opaque_record;
use transfer_legacy_crypto_core::{hash::sha256, jcs::canonicalize, signatures::verify_ed25519};
use transfer_legacy_shared_types::{CURRENT_CRYPTO_VERSION, CURRENT_SCHEMA_VERSION};

#[derive(Debug, Deserialize)]
pub struct CreateItemRequest {
    pub user_id: Uuid,
    pub ciphertext: String,
    pub item_meta: Option<serde_json::Value>,
    pub crypto_version: String,
}

#[derive(Debug, Serialize)]
pub struct CreateItemResponse {
    pub item_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ListItemsRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ItemSummary {
    pub item_id: Uuid,
    pub ciphertext: String,
    pub item_meta: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListItemsResponse {
    pub items: Vec<ItemSummary>,
}

#[derive(Debug, Deserialize)]
pub struct GetItemRequest {
    pub user_id: Uuid,
    pub item_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct GetItemResponse {
    pub item_id: Uuid,
    pub ciphertext: String,
    pub item_meta: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteItemRequest {
    pub user_id: Uuid,
    pub item_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct DeleteItemResponse {
    pub status: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct CreateShareRequest {
    pub owner_id: Uuid,
    pub item_id: Uuid,
    pub grantee_id: Uuid,
    pub envelope: serde_json::Value,
    pub grant_sig: String,
    pub crypto_version: String,
}

#[derive(Debug, Serialize)]
pub struct CreateShareResponse {
    pub share_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ListSharesRequest {
    pub owner_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ShareSummary {
    pub share_id: Uuid,
    pub item_id: Uuid,
    pub grantee_id: Uuid,
    pub envelope: serde_json::Value,
    pub grant_sig: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListSharesResponse {
    pub shares: Vec<ShareSummary>,
}

#[derive(Debug, Deserialize)]
pub struct RevokeShareRequest {
    pub owner_id: Uuid,
    pub share_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct RevokeShareResponse {
    pub status: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct MigrateRequest {
    pub user_id: Uuid,
    pub from_version: String,
    pub to_version: String,
    pub item_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct MigrateResponse {
    pub status: &'static str,
}

pub async fn create_item(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<CreateItemRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;

    if payload.crypto_version != CURRENT_CRYPTO_VERSION.as_str() {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::CryptoVersionUnsupported, &request_id));
    }

    let ciphertext = URL_SAFE_NO_PAD.decode(payload.ciphertext)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id))?;

    let item_id = insert_item(
        &state.db,
        payload.user_id,
        ciphertext,
        payload.item_meta,
        payload.crypto_version,
        CURRENT_SCHEMA_VERSION,
    )
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: CreateItemResponse { item_id },
        request_id: crate::middleware::request_id::request_id_string(&request_id),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}

pub async fn list_items_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<ListItemsRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let items = list_items(&state.db, payload.user_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let list = items
        .into_iter()
        .map(|item| ItemSummary {
            item_id: item.item_id,
            ciphertext: URL_SAFE_NO_PAD.encode(item.ciphertext),
            item_meta: item.item_meta,
            created_at: item.created_at,
        })
        .collect::<Vec<_>>();

    let envelope = crate::errors::SuccessEnvelope {
        data: ListItemsResponse { items: list },
        request_id: crate::middleware::request_id::request_id_string(&request_id),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}

pub async fn get_item_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<GetItemRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let item = get_item(&state.db, payload.user_id, payload.item_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: GetItemResponse {
            item_id: item.item_id,
            ciphertext: URL_SAFE_NO_PAD.encode(item.ciphertext),
            item_meta: item.item_meta,
            created_at: item.created_at,
        },
        request_id: crate::middleware::request_id::request_id_string(&request_id),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}

pub async fn delete_item_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<DeleteItemRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;

    delete_item(&state.db, payload.user_id, payload.item_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: DeleteItemResponse { status: "ok" },
        request_id: crate::middleware::request_id::request_id_string(&request_id),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}

pub async fn create_share(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<CreateShareRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;

    if payload.crypto_version != CURRENT_CRYPTO_VERSION.as_str() {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::CryptoVersionUnsupported, &request_id));
    }

    let envelope_obj = payload.envelope.as_object().ok_or_else(|| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id)
    })?;
    let version = envelope_obj
        .get("crypto_version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id))?;
    if version != CURRENT_CRYPTO_VERSION.as_str() {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::CryptoVersionUnsupported, &request_id));
    }

    let canonical = canonicalize(&payload.envelope)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id))?;
    let digest = sha256(&canonical);

    let grant_sig = URL_SAFE_NO_PAD.decode(payload.grant_sig)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &request_id))?;

    let owner_record = fetch_opaque_record(&state.db, payload.owner_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &request_id))?;
    verify_ed25519(&owner_record.ed25519_pubkey, &digest, &grant_sig)
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::SignatureInvalid, &request_id))?;

    let share_id = insert_share(
        &state.db,
        payload.owner_id,
        payload.item_id,
        payload.grantee_id,
        canonical,
        grant_sig,
        payload.crypto_version,
        CURRENT_SCHEMA_VERSION,
    )
    .await
    .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: CreateShareResponse { share_id },
        request_id: crate::middleware::request_id::request_id_string(&request_id),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}

pub async fn list_shares_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<ListSharesRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let shares = list_shares(&state.db, payload.owner_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let mut list = Vec::with_capacity(shares.len());
    for share in shares {
        let envelope = serde_json::from_slice(&share.envelope)
            .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;
        list.push(ShareSummary {
            share_id: share.share_id,
            item_id: share.item_id,
            grantee_id: share.grantee_id,
            envelope,
            grant_sig: URL_SAFE_NO_PAD.encode(share.grant_sig),
            created_at: share.created_at,
        });
    }

    let envelope = crate::errors::SuccessEnvelope {
        data: ListSharesResponse { shares: list },
        request_id: crate::middleware::request_id::request_id_string(&request_id),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}

pub async fn revoke_share_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<RevokeShareRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;

    revoke_share(&state.db, payload.owner_id, payload.share_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: RevokeShareResponse { status: "ok" },
        request_id: crate::middleware::request_id::request_id_string(&request_id),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}

pub async fn migrate_crypto(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<MigrateRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    require_idempotency(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &request_id))?;

    if payload.to_version != CURRENT_CRYPTO_VERSION.as_str() {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::CryptoVersionUnsupported, &request_id));
    }

    let envelope = crate::errors::SuccessEnvelope {
        data: MigrateResponse { status: "ok" },
        request_id: crate::middleware::request_id::request_id_string(&request_id),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}
