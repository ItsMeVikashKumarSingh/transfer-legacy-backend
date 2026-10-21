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
    OpsCoupon, OpsCreateCouponRequest, OpsCreatePlanRequest, OpsPlan, OpsUpdatePlanRequest,
    ValidateCouponRequest, ValidateCouponResponse,
};
use uuid::Uuid;

// --- PLANS ---

pub async fn list_plans_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<OpsPlan>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let plans = ops_queries::list_plans(&state.db).await.map_err(|e| {
        tracing::error!("Failed to list plans: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    Ok(Json(plans))
}

pub async fn create_plan_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<OpsCreatePlanRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    ops_queries::upsert_plan(
        &state.db,
        &payload.id,
        &payload.name,
        payload.monthly_price,
        payload.yearly_price,
        &payload.description,
        &payload.features,
        payload.max_guardians,
        payload.max_assets,
        payload.highlighted,
        payload.active,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to create plan: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    ops_queries::log_activity(
        &state.db,
        Some(claims.sub),
        "create_plan",
        Some("plan"),
        Some(&payload.id),
        Some(serde_json::json!({ "name": payload.name })),
        None,
    )
    .await
    .ok();

    Ok(Json(serde_json::json!({ "success": true, "id": payload.id })))
}

pub async fn update_plan_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(payload): Json<OpsUpdatePlanRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    let plans = ops_queries::list_plans(&state.db).await.map_err(|e| {
        tracing::error!("Failed to fetch plan: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let existing = plans.into_iter().find(|p| p.id == id).ok_or_else(|| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid)
    })?;

    let name = payload.name.unwrap_or(existing.name);
    let monthly_price = payload.monthly_price.unwrap_or(existing.monthly_price);
    let yearly_price = payload.yearly_price.unwrap_or(existing.yearly_price);
    let description = payload.description.unwrap_or(existing.description);
    let features = payload.features.unwrap_or(existing.features);
    let max_guardians = payload.max_guardians.unwrap_or(existing.max_guardians);
    let max_assets = payload.max_assets.unwrap_or(existing.max_assets);
    let highlighted = payload.highlighted.unwrap_or(existing.highlighted);
    let active = payload.active.unwrap_or(existing.active);

    ops_queries::upsert_plan(
        &state.db,
        &id,
        &name,
        monthly_price,
        yearly_price,
        &description,
        &features,
        max_guardians,
        max_assets,
        highlighted,
        active,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to update plan: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    ops_queries::log_activity(
        &state.db,
        Some(claims.sub),
        "update_plan",
        Some("plan"),
        Some(&id),
        Some(serde_json::json!({ "name": name })),
        None,
    )
    .await
    .ok();

    Ok(Json(serde_json::json!({ "success": true, "id": id })))
}

pub async fn delete_plan_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    ops_queries::delete_plan(&state.db, &id).await.map_err(|e| {
        tracing::error!("Failed to delete plan: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    ops_queries::log_activity(
        &state.db,
        Some(claims.sub),
        "delete_plan",
        Some("plan"),
        Some(&id),
        None,
        None,
    )
    .await
    .ok();

    Ok(Json(serde_json::json!({ "success": true, "deleted_id": id })))
}

// --- COUPONS ---

pub async fn list_coupons_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<Vec<OpsCoupon>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let coupons = ops_queries::list_coupons(&state.db).await.map_err(|e| {
        tracing::error!("Failed to list coupons: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    Ok(Json(coupons))
}

pub async fn create_coupon_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<OpsCreateCouponRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    let id = ops_queries::create_coupon(
        &state.db,
        &payload.code,
        payload.discount_percent,
        payload.max_uses,
        payload.expires_at,
        payload.description.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to create coupon: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    ops_queries::log_activity(
        &state.db,
        Some(claims.sub),
        "create_coupon",
        Some("coupon"),
        Some(&id.to_string()),
        Some(serde_json::json!({ "code": payload.code })),
        None,
    )
    .await
    .ok();

    Ok(Json(serde_json::json!({ "success": true, "id": id })))
}

pub async fn toggle_coupon_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    let active = ops_queries::toggle_coupon_active(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to toggle coupon: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    ops_queries::log_activity(
        &state.db,
        Some(claims.sub),
        "toggle_coupon",
        Some("coupon"),
        Some(&id.to_string()),
        Some(serde_json::json!({ "active": active })),
        None,
    )
    .await
    .ok();

    Ok(Json(serde_json::json!({ "success": true, "id": id, "active": active })))
}

pub async fn delete_coupon_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    ops_queries::delete_coupon(&state.db, id).await.map_err(|e| {
        tracing::error!("Failed to delete coupon: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    ops_queries::log_activity(
        &state.db,
        Some(claims.sub),
        "delete_coupon",
        Some("coupon"),
        Some(&id.to_string()),
        None,
        None,
    )
    .await
    .ok();

    Ok(Json(serde_json::json!({ "success": true, "deleted_id": id })))
}

// Public endpoint for checkout coupon validation
pub async fn validate_coupon_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Json(payload): Json<ValidateCouponRequest>,
) -> Result<Json<ValidateCouponResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    let (valid, discount_percent, discount_amount, final_amount, message) =
        ops_queries::validate_coupon_db(&state.db, &payload.code, payload.amount)
            .await
            .map_err(|e| {
                tracing::error!("Failed to validate coupon: {:?}", e);
                ApiError::app_with_request_id(
                    transfer_legacy_shared_types::AppError::Internal,
                    &rid,
                )
            })?;

    Ok(Json(ValidateCouponResponse {
        valid,
        discount_percent,
        discount_amount,
        final_amount,
        message,
    }))
}
