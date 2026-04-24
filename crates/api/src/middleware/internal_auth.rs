use axum::http::{HeaderMap, StatusCode, request::Parts};
use axum::middleware::Next;
use axum::response::Response;
use axum::extract::{Request, State};
use crate::state::AppState;
use crate::handlers::ops::auth_utils;

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

pub async fn administrative_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Check for legacy internal token first (backward compatibility)
    if let Ok(()) = ensure_internal_access(&state, &headers).await {
        // If internal token is valid, we inject a "super_admin" claim placeholder
        // This keeps existing tests / internal tools working.
        let placeholder_claims = auth_utils::Claims {
            sub: uuid::Uuid::nil(),
            role: "super_admin".to_string(),
            exp: 0,
            iat: 0,
        };
        request.extensions_mut().insert(placeholder_claims);
        return Ok(next.run(request).await);
    }

    // 2. Check for JWT Authorization header
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .filter(|value| value.starts_with("Bearer "))
        .map(|value| &value[7..]);

    if let Some(token) = auth_header {
        let config = state.config().await;
        let claims = auth_utils::validate_token(token, &config.jwt_secret)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        
        // Inject claims for handlers
        request.extensions_mut().insert(claims);
        return Ok(next.run(request).await);
    }

    Err(StatusCode::UNAUTHORIZED)
}
