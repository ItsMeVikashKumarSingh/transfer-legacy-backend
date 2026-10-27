use axum::extract::{Extension, Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::auth::{fetch_user_avatar, update_user_avatar};
use crate::errors::{success, ApiError, SuccessEnvelope};
use crate::services::b2;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PresignAvatarRequest {
    pub user_id: Uuid,
    pub file_name: String,
    pub content_type: String,
}

#[derive(Debug, Serialize)]
pub struct PresignAvatarResponse {
    pub upload_url: String,
    pub public_url: String,
    pub object_key: String,
}

pub async fn presign_avatar_upload(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(payload): Json<PresignAvatarRequest>,
) -> Result<Json<SuccessEnvelope<PresignAvatarResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    // Validate mime type
    if !payload.content_type.starts_with("image/") {
        return Err(ApiError::app_with_request_id(
            transfer_legacy_shared_types::AppError::BadRequest,
            &rid,
        ));
    }

    let config = state.config().await;
    let safe_ext = match payload.content_type.as_str() {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "png",
    };

    let object_key = format!("avatars/{}/avatar_{}.{}", payload.user_id, Uuid::new_v4(), safe_ext);
    let bucket = &config.b2_public_assets_bucket_name;

    let upload_url = b2::presign_put_to_bucket(&config, bucket, &object_key, &payload.content_type, 900)
        .await
        .map_err(|e| {
            tracing::error!("Failed to generate avatar presigned PUT URL: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    // Determine public URL
    let public_url = if config.b2_endpoint_url.contains("://") {
        let endpoint_clean = config.b2_endpoint_url.trim_start_matches("https://").trim_start_matches("http://");
        format!("https://{}.{}/{}", bucket, endpoint_clean, object_key)
    } else {
        format!("https://{}.{}/{}", bucket, config.b2_endpoint_url, object_key)
    };

    Ok(success(
        &rid,
        PresignAvatarResponse {
            upload_url,
            public_url,
            object_key,
        },
    ))
}

#[derive(Debug, Deserialize)]
pub struct ConfirmAvatarRequest {
    pub user_id: Uuid,
    pub avatar_url: String,
}

#[derive(Debug, Serialize)]
pub struct ConfirmAvatarResponse {
    pub status: &'static str,
    pub avatar_url: String,
}

pub async fn confirm_avatar_upload(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(payload): Json<ConfirmAvatarRequest>,
) -> Result<Json<SuccessEnvelope<ConfirmAvatarResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    update_user_avatar(&state.db, payload.user_id, Some(&payload.avatar_url))
        .await
        .map_err(|e| {
            tracing::error!("Failed to save avatar_url: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    Ok(success(
        &rid,
        ConfirmAvatarResponse {
            status: "success",
            avatar_url: payload.avatar_url,
        },
    ))
}

#[derive(Debug, Deserialize)]
pub struct DeleteAvatarRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct DeleteAvatarResponse {
    pub status: &'static str,
}

pub async fn delete_avatar(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(payload): Json<DeleteAvatarRequest>,
) -> Result<Json<SuccessEnvelope<DeleteAvatarResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    update_user_avatar(&state.db, payload.user_id, None)
        .await
        .map_err(|e| {
            tracing::error!("Failed to remove avatar_url: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    Ok(success(&rid, DeleteAvatarResponse { status: "success" }))
}

#[derive(Debug, Serialize)]
pub struct GetAvatarResponse {
    pub avatar_url: Option<String>,
}

pub async fn get_avatar(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<SuccessEnvelope<GetAvatarResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    let avatar_url = fetch_user_avatar(&state.db, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch avatar_url: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    Ok(success(&rid, GetAvatarResponse { avatar_url }))
}
