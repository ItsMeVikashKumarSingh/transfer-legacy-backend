use crate::db::queries::ops as ops_queries;
use crate::errors::ApiError;
use crate::handlers::ops::auth_utils::Claims;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use tower_http::request_id::RequestId;
use transfer_legacy_shared_types::models::ops::{
    OpsUser, UpdateUserPlanRequest, UpdateUserStatusRequest,
};
use uuid::Uuid;

pub async fn list_users_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<Vec<OpsUser>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let users = ops_queries::list_ops_users(&state.db).await.map_err(|e| {
        tracing::error!("Failed to list ops users: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    Ok(Json(users))
}

pub async fn update_user_status_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateUserStatusRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    ops_queries::update_user_status(&state.db, user_id, &payload.status)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update user status: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    ops_queries::log_activity(
        &state.db,
        Some(claims.sub),
        "update_user_status",
        Some("user"),
        Some(&user_id.to_string()),
        Some(serde_json::json!({ "new_status": payload.status })),
        None,
    )
    .await
    .ok();

    Ok(Json(serde_json::json!({ "success": true, "status": payload.status })))
}

pub async fn update_user_plan_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateUserPlanRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    ops_queries::update_user_plan(&state.db, user_id, &payload.plan)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update user plan: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    ops_queries::log_activity(
        &state.db,
        Some(claims.sub),
        "update_user_plan",
        Some("user"),
        Some(&user_id.to_string()),
        Some(serde_json::json!({ "new_plan": payload.plan })),
        None,
    )
    .await
    .ok();

    Ok(Json(serde_json::json!({ "success": true, "plan": payload.plan })))
}
