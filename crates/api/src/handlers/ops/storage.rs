use axum::extract::{Extension, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use tower_http::request_id::RequestId;

use crate::errors::{ApiError, SuccessEnvelope};
use crate::middleware::aead_transport::{wrap_response, AeadResponse};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PresignedUploadRequest {
    pub file_name: String,
    pub content_type: String,
}

#[derive(Debug, Serialize)]
pub struct PresignedUploadResponse {
    pub upload_url: String,
    pub public_url: String,
}

/// Admin endpoint to get a presigned URL for branding assets (e.g. logos)
pub async fn get_presigned_logo_upload(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    Json(req): Json<PresignedUploadRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    crate::middleware::internal_auth::ensure_internal_access(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid))?;
    
    let config = state.config().await;

    let bucket = &config.b2_public_assets_bucket_name;
    let key = format!("branding/{}", req.file_name);
    
    let upload_url = crate::services::b2::presign_put_to_bucket(
        &config,
        bucket,
        &key,
        &req.content_type,
        3600,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to generate presigned URL: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    // Standard B2 public URL pattern (though in production this should likely be a CDN URL)
    let public_url = format!("https://{}.{}/{}", bucket, config.b2_endpoint_url, key);

    let envelope = SuccessEnvelope {
        data: PresignedUploadResponse {
            upload_url,
            public_url,
        },
        request_id: rid,
    };

    let aead = wrap_response(&config, &headers, &envelope)?;
    Ok(Json(aead))
}
