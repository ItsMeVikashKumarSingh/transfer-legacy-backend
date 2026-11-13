use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::middleware::internal_auth::ensure_internal_access;
use crate::services::sessions::verify_session_token;
pub use crate::services::sessions::AuthenticatedUser;
use crate::state::AppState;

pub async fn user_authentication(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Check for internal administrative token (for internal maintenance / tests)
    if let Ok(()) = ensure_internal_access(&state, &headers).await {
        let user_id = headers
            .get("x-user-id")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| Uuid::parse_str(v).ok())
            .unwrap_or(Uuid::nil());
        request
            .extensions_mut()
            .insert(AuthenticatedUser { user_id });
        return Ok(next.run(request).await);
    }

    // 2. Validate Bearer JWT token
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .filter(|value| value.starts_with("Bearer "))
        .map(|value| &value[7..]);

    let token = match auth_header {
        Some(t) => t,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let config = state.config().await;
    let auth_user = verify_session_token(&config, token).map_err(|_| StatusCode::UNAUTHORIZED)?;

    request.extensions_mut().insert(auth_user);
    Ok(next.run(request).await)
}
