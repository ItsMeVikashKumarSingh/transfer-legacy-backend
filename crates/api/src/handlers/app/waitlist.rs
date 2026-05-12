use axum::extract::{Extension, State};
use axum::http::HeaderMap;
use axum::Json;
use tower_http::request_id::RequestId;
use serde_json::json;

use crate::db::queries::app::{insert_waitlist_signup, list_waitlist_entries};
use crate::errors::{ApiError, success, SuccessEnvelope};
use crate::middleware::aead_transport::{wrap_response, AeadResponse};
use crate::state::AppState;
use crate::notifications::resend::NotificationTemplate;
use transfer_legacy_shared_types::models::app::WaitlistSignupRequest;

/// Public endpoint for waitlist signup
/// Captures industry-standard metadata (UTMs, User-Agent, Referrer)
pub async fn waitlist_signup(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    Json(mut req): Json<WaitlistSignupRequest>,
) -> Result<Json<SuccessEnvelope<serde_json::Value>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    // ... (omitting meta extraction for brevity, keeping existing logic)
    let mut meta = json!({
        "user_agent": headers.get("user-agent").and_then(|v| v.to_str().ok()),
        "referrer": headers.get("referer").and_then(|v| v.to_str().ok()),
        "accept_language": headers.get("accept-language").and_then(|v| v.to_str().ok()),
        "ip_country": headers.get("cf-ipcountry").and_then(|v| v.to_str().ok()),
        "captured_at": chrono::Utc::now(),
    });

    if let Some(req_meta) = req.metadata.as_mut() {
        if let Some(incoming_obj) = req_meta.as_object_mut() {
            if let Some(final_obj) = meta.as_object_mut() {
                for (k, v) in incoming_obj.into_iter() {
                    final_obj.insert(k.clone(), v.clone());
                }
            }
        }
    }
    
    req.metadata = Some(meta);
    let name_opt = req.name.clone();
    let email = req.email.clone();

    let (_id, position, is_new) = insert_waitlist_signup(&state.db, req).await.map_err(|e| {
        tracing::error!("Waitlist signup failed: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    if is_new {
        // Enqueue welcome email
        let _ = state.notify(
            uuid::Uuid::nil(),
            &email,
            NotificationTemplate::WaitlistWelcome {
                owner_name: name_opt.unwrap_or_else(|| "there".to_string()),
                position,
            }
        ).await;
    }

    let message = if is_new {
        "Successfully joined waitlist"
    } else {
        "You are already on the waitlist!"
    };

    Ok(Json(SuccessEnvelope {
        data: json!({
            "message": message,
            "position": position,
            "isNew": is_new
        }),
        request_id: rid
    }))
}

/// Protected endpoint to list waitlist entries
/// Requires Internal Token and AEAD wrapping
pub async fn list_waitlist_entries_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    
    crate::middleware::internal_auth::ensure_internal_access(&state, &headers)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid))?;
    
    let entries = list_waitlist_entries(&state.db).await.map_err(|e| {
        tracing::error!("Failed to list waitlist entries: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let envelope = SuccessEnvelope {
        data: entries,
        request_id: rid,
    };
    
    let config_state = state.config().await;
    let aead = wrap_response(&config_state, &headers, &envelope)?;
    Ok(Json(aead))
}
