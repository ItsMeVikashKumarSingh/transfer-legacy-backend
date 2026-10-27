use axum::extract::{Extension, State};
use axum::Json;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::vault::{
    delete_item_attachment, get_item_attachment, insert_item_attachment, list_item_attachments,
};
use crate::errors::{success, ApiError, SuccessEnvelope};
use crate::services::b2;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PresignItemAttachmentRequest {
    pub user_id: Uuid,
    pub item_id: Uuid,
    pub file_name: String,
    pub content_type: String,
}

#[derive(Debug, Serialize)]
pub struct PresignItemAttachmentResponse {
    pub attachment_id: Uuid,
    pub upload_url: String,
    pub object_key: String,
}

pub async fn presign_attachment_upload(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(payload): Json<PresignItemAttachmentRequest>,
) -> Result<Json<SuccessEnvelope<PresignItemAttachmentResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    let attachment_id = Uuid::new_v4();
    let object_key = format!("vault/{}/{}/{}.enc", payload.user_id, payload.item_id, attachment_id);

    let upload_url = b2::presign_put(&config, &object_key, &payload.content_type, 900)
        .await
        .map_err(|e| {
            tracing::error!("Failed to generate vault presigned PUT: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    Ok(success(
        &rid,
        PresignItemAttachmentResponse {
            attachment_id,
            upload_url,
            object_key,
        },
    ))
}

#[derive(Debug, Deserialize)]
pub struct ConfirmItemAttachmentRequest {
    pub user_id: Uuid,
    pub item_id: Uuid,
    pub attachment_id: Uuid,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub sha256_b64: Option<String>,
    pub object_key: String,
    pub is_encrypted: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ConfirmItemAttachmentResponse {
    pub status: &'static str,
    pub attachment_id: Uuid,
}

pub async fn confirm_attachment_upload(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(payload): Json<ConfirmItemAttachmentRequest>,
) -> Result<Json<SuccessEnvelope<ConfirmItemAttachmentResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    let sha256 = if let Some(b64) = payload.sha256_b64 {
        URL_SAFE_NO_PAD.decode(b64).ok()
    } else {
        None
    };

    insert_item_attachment(
        &state.db,
        payload.attachment_id,
        payload.item_id,
        payload.user_id,
        &payload.file_name,
        &payload.mime_type,
        payload.size_bytes,
        sha256,
        &payload.object_key,
        payload.is_encrypted.unwrap_or(true),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to record item attachment: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    Ok(success(
        &rid,
        ConfirmItemAttachmentResponse {
            status: "confirmed",
            attachment_id: payload.attachment_id,
        },
    ))
}

#[derive(Debug, Deserialize)]
pub struct ListItemAttachmentsRequest {
    pub user_id: Uuid,
    pub item_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ItemAttachmentInfo {
    pub attachment_id: Uuid,
    pub item_id: Uuid,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub is_encrypted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListItemAttachmentsResponse {
    pub attachments: Vec<ItemAttachmentInfo>,
}

pub async fn list_attachments(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(payload): Json<ListItemAttachmentsRequest>,
) -> Result<Json<SuccessEnvelope<ListItemAttachmentsResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    let rows = list_item_attachments(&state.db, payload.item_id, payload.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to query item attachments: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    let attachments = rows
        .into_iter()
        .map(|r| ItemAttachmentInfo {
            attachment_id: r.attachment_id,
            item_id: r.item_id,
            file_name: r.file_name,
            mime_type: r.mime_type,
            size_bytes: r.size_bytes,
            is_encrypted: r.is_encrypted,
            created_at: r.created_at,
        })
        .collect();

    Ok(success(&rid, ListItemAttachmentsResponse { attachments }))
}

#[derive(Debug, Deserialize)]
pub struct DownloadItemAttachmentRequest {
    pub user_id: Uuid,
    pub attachment_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct DownloadItemAttachmentResponse {
    pub download_url: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub is_encrypted: bool,
}

pub async fn download_attachment(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(payload): Json<DownloadItemAttachmentRequest>,
) -> Result<Json<SuccessEnvelope<DownloadItemAttachmentResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    let attachment = get_item_attachment(&state.db, payload.attachment_id, payload.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Attachment not found or unauthorized: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid)
        })?;

    let download_url = b2::presign_get(&config, &attachment.object_key, 900)
        .await
        .map_err(|e| {
            tracing::error!("Failed to generate presigned GET URL: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    Ok(success(
        &rid,
        DownloadItemAttachmentResponse {
            download_url,
            file_name: attachment.file_name,
            mime_type: attachment.mime_type,
            size_bytes: attachment.size_bytes,
            is_encrypted: attachment.is_encrypted,
        },
    ))
}

#[derive(Debug, Deserialize)]
pub struct DeleteItemAttachmentRequest {
    pub user_id: Uuid,
    pub attachment_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct DeleteItemAttachmentResponse {
    pub status: &'static str,
}

pub async fn delete_attachment(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(payload): Json<DeleteItemAttachmentRequest>,
) -> Result<Json<SuccessEnvelope<DeleteItemAttachmentResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    delete_item_attachment(&state.db, payload.attachment_id, payload.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete item attachment: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    Ok(success(&rid, DeleteItemAttachmentResponse { status: "deleted" }))
}
