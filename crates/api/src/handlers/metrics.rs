use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;

use crate::state::AppState;

pub async fn metrics(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    crate::middleware::internal_auth::ensure_internal_access(&state, &headers).await?;
    Ok(crate::telemetry::render_metrics())
}
