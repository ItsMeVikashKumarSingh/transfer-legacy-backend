use axum::extract::{Extension, State};
use axum::http::HeaderMap;
use axum::Json;
use tower_http::request_id::RequestId;

use crate::db::queries::app::{fetch_branding, update_branding};
use crate::errors::{ApiError, success, SuccessEnvelope};
use crate::middleware::aead_transport::{wrap_response, AeadResponse, AeadJson};
use crate::state::AppState;
use transfer_legacy_shared_types::models::app::BrandingConfig;

/// Public endpoint to fetch site branding
pub async fn get_branding(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<SuccessEnvelope<BrandingConfig>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let branding = fetch_branding(&state.db).await.map_err(|e| {
        tracing::error!("Failed to fetch branding: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    Ok(success(&rid, branding))
}

/// Protected endpoint to update site branding
/// Requires Internal Token and AEAD wrapping
pub async fn update_branding_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    AeadJson(config): AeadJson<BrandingConfig>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    update_branding(&state.db, config).await.map_err(|e| {
        tracing::error!("Failed to update branding: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let envelope = SuccessEnvelope {
        data: "Branding updated successfully".to_string(),
        request_id: rid,
    };
    
    let config_state = state.config().await;
    let aead = wrap_response(&config_state, &headers, &envelope)?;
    Ok(Json(aead))
}
