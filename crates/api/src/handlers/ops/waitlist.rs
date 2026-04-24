use axum::{extract::State, Json, Extension};
use tower_http::request_id::RequestId;
use crate::errors::ApiError;
use crate::state::AppState;
use crate::db::queries::app as app_queries;
use crate::handlers::ops::auth_utils::Claims;
use transfer_legacy_shared_types::models::app::WaitlistEntry;

pub async fn list_waitlist_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<Vec<WaitlistEntry>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    
    let entries = app_queries::list_waitlist_entries(&state.db).await.map_err(|e| {
        tracing::error!("Failed to list waitlist: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    Ok(Json(entries))
}
