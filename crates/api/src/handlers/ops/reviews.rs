use axum::extract::{Extension, Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::ops::{fetch_manual_review, list_manual_reviews};
use crate::errors::ApiError;
use crate::middleware::aead_transport::{AeadResponse, wrap_response};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListReviewsQuery {
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReviewSummary {
    pub review_id: Uuid,
    pub policy_id: Uuid,
    pub conflict_id: Option<Uuid>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ReviewDetail {
    pub review_id: Uuid,
    pub policy_id: Uuid,
    pub conflict_id: Option<Uuid>,
    pub status: String,
    pub notes: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn list_reviews(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Query(query): Query<ListReviewsQuery>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rows = list_manual_reviews(&state.db, query.status.as_deref())
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let data: Vec<ReviewSummary> = rows
        .into_iter()
        .map(|r| ReviewSummary {
            review_id: r.review_id,
            policy_id: r.policy_id,
            conflict_id: r.conflict_id,
            status: r.status,
            created_at: r.created_at,
            resolved_at: r.resolved_at,
        })
        .collect();

    let envelope = crate::errors::SuccessEnvelope {
        data,
        request_id: crate::middleware::request_id::request_id_string(&request_id),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}

pub async fn get_review(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Path(review_id): Path<Uuid>,
) -> Result<Json<AeadResponse>, ApiError> {
    let row = fetch_manual_review(&state.db, review_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &request_id))?;

    let envelope = crate::errors::SuccessEnvelope {
        data: ReviewDetail {
            review_id: row.review_id,
            policy_id: row.policy_id,
            conflict_id: row.conflict_id,
            status: row.status,
            notes: row.notes,
            created_at: row.created_at,
            resolved_at: row.resolved_at,
        },
        request_id: crate::middleware::request_id::request_id_string(&request_id),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}
