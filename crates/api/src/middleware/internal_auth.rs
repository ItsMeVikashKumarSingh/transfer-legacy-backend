use axum::http::{HeaderMap, StatusCode};

use crate::state::AppState;

pub fn ensure_internal_access(state: &AppState, headers: &HeaderMap) -> Result<(), StatusCode> {
    match &state.config.internal_api_token {
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
