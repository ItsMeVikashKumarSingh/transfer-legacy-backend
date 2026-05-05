use axum::{extract::State, Json, Extension};
use tower_http::request_id::RequestId;
use crate::errors::ApiError;
use crate::state::AppState;
use crate::db::queries::app as app_queries;
use crate::db::queries::ops as ops_queries;
use crate::handlers::ops::auth_utils::Claims;
use transfer_legacy_shared_types::models::app::BrandingConfig;

pub async fn get_branding_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<BrandingConfig>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = app_queries::fetch_branding(&state.db).await.map_err(|e| {
        tracing::error!("Failed to fetch branding: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    Ok(Json(config))
}

pub async fn update_branding_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
    Json(payload): Json<BrandingConfig>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    app_queries::update_branding(&state.db, payload).await.map_err(|e| {
        tracing::error!("Failed to update branding: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    // Log activity
    let _ = ops_queries::log_activity(
        &state.db,
        Some(_claims.sub),
        "update_branding",
        Some("settings"),
        Some("1"),
        None,
        None
    ).await;

    Ok(Json(serde_json::json!({ "status": "updated" })))
}

pub async fn update_content_ops_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
    Json(content): Json<transfer_legacy_shared_types::models::app::AppContent>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    
    app_queries::update_app_content(&state.db, content.clone()).await.map_err(|e| {
        tracing::error!("Failed to update app content from ops: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    // Log activity
    let _ = ops_queries::log_activity(
        &state.db,
        Some(_claims.sub),
        "update_content",
        Some("content"),
        Some(&content.slug),
        None,
        None
    ).await;

    Ok(Json(serde_json::json!({ "status": "updated" })))
}
