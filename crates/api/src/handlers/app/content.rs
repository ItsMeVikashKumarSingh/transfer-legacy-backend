use axum::extract::{Extension, Path, State};
use axum::http::HeaderMap;
use axum::Json;
use tower_http::request_id::RequestId;

use crate::db::queries::app::{fetch_app_content, update_app_content};
use crate::errors::{ApiError, success, SuccessEnvelope};
use crate::middleware::aead_transport::{wrap_response, AeadResponse, AeadJson};
use crate::state::AppState;
use transfer_legacy_shared_types::models::app::AppContent;

/// Public endpoint to fetch dynamic page content (e.g., hero, team, faqs)
pub async fn get_content(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Path(slug): Path<String>,
) -> Result<Json<SuccessEnvelope<AppContent>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let content = fetch_app_content(&state.db, &slug).await.map_err(|e| {
        tracing::error!("Failed to fetch app content for slug {}: {:?}", slug, e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid)
    })?;

    Ok(success(&rid, content))
}

/// Protected endpoint to update dynamic page content
/// Requires Internal Token and AEAD wrapping
pub async fn update_content_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    AeadJson(content): AeadJson<AppContent>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    crate::middleware::internal_auth::ensure_internal_access(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid))?;
    
    update_app_content(&state.db, content).await.map_err(|e| {
        tracing::error!("Failed to update app content: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let envelope = SuccessEnvelope {
        data: "Content updated successfully".to_string(),
        request_id: rid,
    };
    
    let config_state = state.config().await;
    let aead = wrap_response(&config_state, &headers, &envelope)?;
    Ok(Json(aead))
}
