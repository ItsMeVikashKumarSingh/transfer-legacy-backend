use axum::{extract::{State, Path}, Json, Extension};
use uuid::Uuid;
use tower_http::request_id::RequestId;
use crate::errors::ApiError;
use crate::state::AppState;
use crate::db::queries::ops as ops_queries;
use crate::handlers::ops::auth_utils::{self, Claims};
use transfer_legacy_shared_types::models::ops::{
    OpsAdmin, OpsRole, OpsCreateAdminRequest, OpsUpdateAdminRequest
};

pub async fn list_admins_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<Vec<OpsAdmin>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let admins = ops_queries::list_admins(&state.db).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    Ok(Json(admins))
}

pub async fn create_admin_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<OpsCreateAdminRequest>,
) -> Result<Json<Uuid>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    if claims.role != "super_admin" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid));
    }

    let password_hash = auth_utils::hash_password(&payload.password).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    
    let id = ops_queries::create_admin(&state.db, &payload.email, &password_hash, payload.role_id).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    ops_queries::log_activity(
        &state.db, 
        Some(claims.sub), 
        "create_admin", 
        Some("admin"), 
        Some(&id.to_string()), 
        Some(serde_json::json!({ "email": payload.email })), 
        None
    ).await.ok();

    Ok(Json(id))
}

pub async fn delete_admin_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Path(admin_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    if claims.sub == admin_id {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid));
    }

    if claims.role != "super_admin" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid));
    }

    ops_queries::delete_admin(&state.db, admin_id).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    
    ops_queries::log_activity(
        &state.db, 
        Some(claims.sub), 
        "delete_admin", 
        Some("admin"), 
        Some(&admin_id.to_string()), 
        None, 
        None
    ).await.ok();

    Ok(Json(serde_json::json!({ "status": "deleted" })))
}

pub async fn list_roles_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<Vec<OpsRole>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let roles = ops_queries::list_roles(&state.db).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    Ok(Json(roles))
}

pub async fn create_role_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<OpsRole>,
) -> Result<Json<Uuid>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    if claims.role != "super_admin" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid));
    }

    let id = ops_queries::create_role(&state.db, &payload.name, payload.description.as_deref(), payload.permissions).await.map_err(|e| {
        tracing::error!("Create role failed: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    
    ops_queries::log_activity(
        &state.db, 
        Some(claims.sub), 
        "create_role", 
        Some("role"), 
        Some(&id.to_string()), 
        Some(serde_json::json!({ "name": payload.name })), 
        None
    ).await.ok();

    Ok(Json(id))
}

pub async fn delete_role_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Path(role_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    if claims.role != "super_admin" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid));
    }

    ops_queries::delete_role(&state.db, role_id).await.map_err(|e| {
        tracing::error!("Delete role failed: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    
    ops_queries::log_activity(
        &state.db, 
        Some(claims.sub), 
        "delete_role", 
        Some("role"), 
        Some(&role_id.to_string()), 
        None, 
        None
    ).await.ok();

    Ok(Json(serde_json::json!({ "status": "deleted" })))
}

pub async fn update_role_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Path(role_id): Path<Uuid>,
    Json(payload): Json<OpsRole>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    if claims.role != "super_admin" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid));
    }

    ops_queries::update_role(
        &state.db, 
        role_id, 
        &payload.name, 
        payload.description.as_deref(), 
        payload.permissions
    ).await.map_err(|e| {
        tracing::error!("Update role failed: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    
    ops_queries::log_activity(
        &state.db, 
        Some(claims.sub), 
        "update_role", 
        Some("role"), 
        Some(&role_id.to_string()), 
        Some(serde_json::json!({ "name": payload.name })), 
        None
    ).await.ok();

    Ok(Json(serde_json::json!({ "status": "updated" })))
}
