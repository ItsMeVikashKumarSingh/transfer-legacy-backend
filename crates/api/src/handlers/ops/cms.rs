use axum::{extract::{State, Path}, Json, Extension};
use tower_http::request_id::RequestId;
use crate::errors::ApiError;
use crate::state::AppState;
use crate::db::queries::app as app_queries;
use crate::db::queries::ops as ops_queries;
use crate::handlers::ops::auth_utils::Claims;
use transfer_legacy_shared_types::models::app::CmsPage;

pub async fn list_pages_ops(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<Vec<CmsPage>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    
    let pages = app_queries::list_cms_pages(&state.db, false)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;
        
    Ok(Json(pages))
}

pub async fn get_page_ops(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
    Path(slug): Path<String>,
) -> Result<Json<CmsPage>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    
    let page = app_queries::fetch_cms_page_by_slug(&state.db, &slug, false)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid)
        })?;
        
    Ok(Json(page))
}

pub async fn upsert_page_ops(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Path(slug): Path<String>,
    Json(mut page): Json<CmsPage>,
) -> Result<Json<()>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    
    page.slug = slug.clone();
    page.updated_by = Some(claims.sub.to_string());
    
    app_queries::upsert_cms_page(&state.db, page)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    // Log activity
    let _ = ops_queries::log_activity(
        &state.db,
        Some(claims.sub),
        "update_cms",
        Some("cms_pages"),
        Some(&slug),
        None,
        None
    ).await;
        
    Ok(Json(()))
}

pub async fn delete_page_ops(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(claims): Extension<Claims>,
    Path(slug): Path<String>,
) -> Result<Json<()>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    
    app_queries::delete_cms_page(&state.db, &slug)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    // Log activity
    let _ = ops_queries::log_activity(
        &state.db,
        Some(claims.sub),
        "delete_cms",
        Some("cms_pages"),
        Some(&slug),
        None,
        None
    ).await;
        
    Ok(Json(()))
}
