use axum::{extract::{State, Path}, Json, Extension};
use uuid::Uuid;
use tower_http::request_id::RequestId;
use crate::errors::ApiError;
use crate::state::AppState;
use crate::db::queries::app as app_queries;
use crate::db::queries::ops as ops_queries;
use crate::handlers::ops::auth_utils::Claims;
use transfer_legacy_shared_types::models::app::{ContactConfig, ContactMessage};

pub async fn get_contact_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<ContactConfig>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = app_queries::fetch_contact_config(&state.db).await.map_err(|e| {
        tracing::error!("Failed to fetch contact config: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    Ok(Json(config))
}

pub async fn update_contact_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
    Json(payload): Json<ContactConfig>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    app_queries::update_contact_config(&state.db, payload).await.map_err(|e| {
        tracing::error!("Failed to update contact config: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    // Log activity
    let _ = ops_queries::log_activity(
        &state.db,
        Some(_claims.sub),
        "update_contact_config",
        Some("contact_config"),
        Some("1"),
        None,
        None
    ).await;

    Ok(Json(serde_json::json!({ "status": "updated" })))
}

pub async fn list_contact_messages_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<Vec<ContactMessage>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let messages = app_queries::list_contact_messages(&state.db).await.map_err(|e| {
        tracing::error!("Failed to list contact messages: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    Ok(Json(messages))
}

pub async fn delete_contact_message_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
    Path(message_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    app_queries::delete_contact_message(&state.db, message_id).await.map_err(|e| {
        tracing::error!("Failed to delete contact message: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    // Log activity
    let _ = ops_queries::log_activity(
        &state.db,
        Some(_claims.sub),
        "delete_contact_message",
        Some("contact_messages"),
        Some(&message_id.to_string()),
        None,
        None
    ).await;
    Ok(Json(serde_json::json!({ "status": "deleted" })))
}
