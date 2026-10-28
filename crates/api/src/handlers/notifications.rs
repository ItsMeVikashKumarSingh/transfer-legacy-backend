use axum::extract::{Extension, Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::notify::{
    count_unread_notifications, create_user_notification, delete_user_notification,
    get_or_create_notification_preferences, list_user_notifications, mark_all_notifications_read,
    mark_notification_read, update_notification_preferences, NotificationPreferencesRow,
    UserNotificationRow,
};
use crate::errors::{success, ApiError, SuccessEnvelope};
use crate::notifications::onesignal::{send_push_notification, PushNotificationPayload};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListNotificationsQuery {
    pub user_id: Uuid,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub unread_only: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct NotificationListResponse {
    pub notifications: Vec<UserNotificationRow>,
    pub unread_count: i64,
}

pub async fn list_notifications(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Query(params): Query<ListNotificationsQuery>,
) -> Result<Json<SuccessEnvelope<NotificationListResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let limit = params.limit.unwrap_or(50).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);
    let unread_only = params.unread_only.unwrap_or(false);

    let notifications = list_user_notifications(&state.db, params.user_id, limit, offset, unread_only)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch user notifications: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    let unread_count = count_unread_notifications(&state.db, params.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count unread notifications: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    Ok(success(
        &rid,
        NotificationListResponse {
            notifications,
            unread_count,
        },
    ))
}

#[derive(Debug, Deserialize)]
pub struct UserActionRequest {
    pub user_id: Uuid,
}

pub async fn mark_read(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Path(notification_id): Path<Uuid>,
    Json(payload): Json<UserActionRequest>,
) -> Result<Json<SuccessEnvelope<bool>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    let updated = mark_notification_read(&state.db, notification_id, payload.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to mark notification read: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    Ok(success(&rid, updated))
}

pub async fn mark_all_read(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(payload): Json<UserActionRequest>,
) -> Result<Json<SuccessEnvelope<u64>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    let count = mark_all_notifications_read(&state.db, payload.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to mark all notifications read: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    Ok(success(&rid, count))
}

pub async fn delete_notification(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Path(notification_id): Path<Uuid>,
    Query(params): Query<UserActionRequest>,
) -> Result<Json<SuccessEnvelope<bool>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    let deleted = delete_user_notification(&state.db, notification_id, params.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete notification: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    Ok(success(&rid, deleted))
}

#[derive(Debug, Deserialize)]
pub struct GetPreferencesQuery {
    pub user_id: Uuid,
}

pub async fn get_preferences(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Query(params): Query<GetPreferencesQuery>,
) -> Result<Json<SuccessEnvelope<NotificationPreferencesRow>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    let prefs = get_or_create_notification_preferences(&state.db, params.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get notification preferences: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    Ok(success(&rid, prefs))
}

#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub user_id: Uuid,
    pub email_enabled: bool,
    pub push_enabled: bool,
    pub in_app_enabled: bool,
    pub security_alerts: bool,
    pub heartbeat_reminders: bool,
    pub guardian_signals: bool,
    pub vault_updates: bool,
}

pub async fn update_preferences(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(payload): Json<UpdatePreferencesRequest>,
) -> Result<Json<SuccessEnvelope<NotificationPreferencesRow>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    let prefs = update_notification_preferences(
        &state.db,
        payload.user_id,
        payload.email_enabled,
        payload.push_enabled,
        payload.in_app_enabled,
        payload.security_alerts,
        payload.heartbeat_reminders,
        payload.guardian_signals,
        payload.vault_updates,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to update notification preferences: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    Ok(success(&rid, prefs))
}

#[derive(Debug, Deserialize)]
pub struct TestPushRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct TestPushResponse {
    pub in_app_created: bool,
    pub push_dispatched: bool,
    pub message: String,
}

pub async fn test_push(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    Json(payload): Json<TestPushRequest>,
) -> Result<Json<SuccessEnvelope<TestPushResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    // 1. Create real in-app notification
    let in_app_result = create_user_notification(
        &state.db,
        payload.user_id,
        "OneSignal Handshake Verified",
        "Your institutional signal channel is active. Real-time legacy updates will be dispatched here.",
        "security",
        "Bell",
        Some("/settings"),
        Some(serde_json::json!({ "source": "test_signal" })),
    )
    .await;

    let in_app_created = in_app_result.is_ok();

    // 2. Dispatch real OneSignal Web Push
    let config = state.config().await;
    let push_payload = PushNotificationPayload {
        user_id: payload.user_id,
        title: "Transfer Legacy Handshake".to_string(),
        message: "Your signal receiver is verified. Institutional protocol alerts are enabled.".to_string(),
        action_url: Some(format!("{}/settings", config.app_url)),
        data: Some(serde_json::json!({
            "type": "verification_test",
            "ts": chrono::Utc::now().timestamp(),
        })),
    };

    let push_dispatched = send_push_notification(&config, &push_payload)
        .await
        .unwrap_or(false);

    let message = if push_dispatched {
        "Test push sent via OneSignal and logged to your in-app inbox.".to_string()
    } else {
        "Logged to your in-app inbox. (OneSignal push skipped: update ONESIGNAL_APP_ID and ONESIGNAL_REST_API_KEY in .env.local to deliver to browser)".to_string()
    };

    Ok(success(
        &rid,
        TestPushResponse {
            in_app_created,
            push_dispatched,
            message,
        },
    ))
}
