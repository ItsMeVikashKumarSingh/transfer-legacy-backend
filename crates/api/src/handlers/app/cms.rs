use axum::{extract::{State, Path}, Json, Extension};
use tower_http::request_id::RequestId;
use crate::errors::ApiError;
use crate::state::AppState;
use crate::db::queries::app as app_queries;
use transfer_legacy_shared_types::models::app::CmsPage;

pub async fn list_pages(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<Vec<CmsPage>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    
    let pages = app_queries::list_cms_pages(&state.db, true)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;
        
    Ok(Json(pages))
}

pub async fn get_page(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Path(slug): Path<String>,
) -> Result<Json<CmsPage>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    
    let page = app_queries::fetch_cms_page_by_slug(&state.db, &slug, true)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid)
        })?;
        
    Ok(Json(page))
}
