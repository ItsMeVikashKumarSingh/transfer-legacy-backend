use axum::{extract::State, Json, Extension};
use uuid::Uuid;
use tower_http::request_id::RequestId;
use crate::errors::ApiError;
use crate::state::AppState;
use crate::db::queries::ops as ops_queries;
use crate::handlers::ops::auth_utils;
use transfer_legacy_shared_types::models::ops::{
    OpsAdmin, OpsLoginRequest, OpsLoginResponse, OpsChangePasswordRequest
};

pub async fn login_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Json(payload): Json<OpsLoginRequest>,
) -> Result<Json<OpsLoginResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    // 1. Fallback Logic (similar to temp backend)
    if payload.email.to_lowercase() == config.ops_admin_email.to_lowercase() 
       && !config.ops_admin_password.is_empty() 
       && payload.password == config.ops_admin_password 
    {
        let admin_id = Uuid::nil();
        let role_name = "super_admin".to_string();
        let token = auth_utils::generate_token(admin_id, &role_name, &config.jwt_secret).map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

        return Ok(Json(OpsLoginResponse {
            token,
            admin: OpsAdmin {
                id: admin_id,
                email: config.ops_admin_email.clone(),
                role_id: Uuid::nil(),
                role_name,
                is_active: true,
                last_login: Some(chrono::Utc::now()),
                created_at: chrono::Utc::now(),
            },
        }));
    }

    // 2. Database Logic
    let (admin_id, password_hash, role_name) = ops_queries::fetch_admin_by_email(&state.db, &payload.email)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid))?;

    let is_valid = auth_utils::verify_password(&payload.password, &password_hash).map_err(|e| {
         ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    
    if !is_valid {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid));
    }

    let token = auth_utils::generate_token(admin_id, &role_name, &config.jwt_secret).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    // Log the login
    ops_queries::update_last_login(&state.db, admin_id).await.ok();
    ops_queries::log_activity(
        &state.db, 
        Some(admin_id), 
        "login", 
        Some("admin"), 
        Some(&admin_id.to_string()), 
        None, 
        None
    ).await.ok();

    // Fetch the full admin record for the response
    let admins = ops_queries::list_admins(&state.db).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    
    let admin = admins.into_iter().find(|a| a.id == admin_id)
        .ok_or_else(|| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid))?;

    Ok(Json(OpsLoginResponse { token, admin }))
}

pub async fn change_password_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<auth_utils::Claims>,
    Json(payload): Json<OpsChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    // 1. Fetch current admin details
    let password_hash = ops_queries::get_admin_password_hash(&state.db, claims.sub).await.map_err(|_| {
         ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid)
    })?;
    
    let is_valid = auth_utils::verify_password(&payload.current_password, &password_hash).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    
    if !is_valid {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid));
    }

    // 3. Hash new password and update
    let new_hash = auth_utils::hash_password(&payload.new_password).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    
    ops_queries::update_admin_password(&state.db, claims.sub, &new_hash).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    ops_queries::log_activity(
        &state.db, 
        Some(claims.sub), 
        "change_password", 
        Some("admin"), 
        Some(&claims.sub.to_string()), 
        None, 
        None
    ).await.ok();

    Ok(Json(serde_json::json!({ "status": "success", "message": "Password updated successfully" })))
}
