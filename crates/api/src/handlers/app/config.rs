use axum::extract::{Extension, State};
use axum::http::HeaderMap;
use axum::Json;
use tower_http::request_id::RequestId;

use crate::db::queries::app::{fetch_branding, update_branding, fetch_contact_config, insert_contact_message};
use crate::errors::{ApiError, success, SuccessEnvelope};
use crate::middleware::aead_transport::{wrap_response, AeadResponse, AeadJson};
use crate::state::AppState;
use transfer_legacy_shared_types::models::app::{BrandingConfig, ContactConfig, ContactMessage};
use uuid::Uuid;

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

#[derive(serde::Deserialize)]
pub struct PublicContactMessageRequest {
    pub name: String,
    pub email: String,
    pub subject: Option<String>,
    pub message: String,
}

pub async fn get_contact_config_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<ContactConfig>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let contact = fetch_contact_config(&state.db).await.map_err(|e| {
        tracing::error!("Failed to fetch contact config: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    Ok(Json(contact))
}

pub async fn submit_contact_message_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Json(payload): Json<PublicContactMessageRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    
    let msg = ContactMessage {
        id: Uuid::new_v4(),
        name: payload.name,
        email: payload.email,
        subject: payload.subject,
        message: payload.message,
        metadata: None,
        is_read: false,
        created_at: chrono::Utc::now(),
    };

    insert_contact_message(&state.db, msg).await.map_err(|e| {
        tracing::error!("Failed to save contact message: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    Ok(Json(serde_json::json!({ "status": "message_sent" })))
}

