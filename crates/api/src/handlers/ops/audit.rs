use crate::db::queries::ops as ops_queries;
use crate::errors::ApiError;
use crate::handlers::ops::auth_utils::Claims;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    Extension, Json,
};
use serde::Deserialize;
use serde_json::Value;
use tower_http::request_id::RequestId;

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub action: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_audit_logs_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(_claims): Extension<Claims>,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<Vec<Value>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let limit = query.limit.unwrap_or(100);

    let logs = ops_queries::list_activity_logs(&state.db, query.action.as_deref(), limit)
        .await
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    Ok(Json(logs))
}
