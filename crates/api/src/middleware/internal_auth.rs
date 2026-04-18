use axum::http::{HeaderMap, StatusCode};

use crate::state::AppState;

pub async fn ensure_internal_access(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), StatusCode> {
    let config = state.config().await;
    match &config.internal_api_token {
        Some(expected) => {
            let provided = headers
                .get("x-internal-token")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            if provided == expected {
                Ok(())
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        None => Ok(()),
    }
}

use axum::middleware::Next;
use axum::response::Response;
use axum::extract::{Request, State};

pub async fn administrative_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    ensure_internal_access(&state, &headers).await?;
    Ok(next.run(request).await)
}
