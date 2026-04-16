use axum::http::HeaderMap;
use redis::AsyncCommands;

use crate::errors::ApiError;
use crate::state::AppState;
use transfer_legacy_shared_types::AppError;

pub async fn require_idempotency(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let key = headers
        .get("x-idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| ApiError::app(AppError::BadRequest))?;

    let mut conn = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| ApiError::app(AppError::Internal))?;
    let redis_key = format!("idem:{}", key);

    let set: bool = conn
        .set_nx(&redis_key, 1)
        .await
        .map_err(|_| ApiError::app(AppError::Internal))?;
    if !set {
        return Err(ApiError::app(AppError::Conflict));
    }
    let _: () = conn
        .expire(&redis_key, 86400)
        .await
        .map_err(|_| ApiError::app(AppError::Internal))?;

    Ok(())
}

pub async fn enforce_rate_limit(
    state: &AppState,
    key: &str,
    max_per_minute: u64,
) -> Result<(), ApiError> {
    let mut conn = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| ApiError::app(AppError::Internal))?;
    let redis_key = format!("rate:{}", key);
    let count: u64 = conn
        .incr(&redis_key, 1)
        .await
        .map_err(|_| ApiError::app(AppError::Internal))?;
    if count == 1 {
        let _: () = conn
            .expire(&redis_key, 60)
            .await
            .map_err(|_| ApiError::app(AppError::Internal))?;
    }
    if count > max_per_minute {
        return Err(ApiError::app(AppError::RateLimited));
    }
    Ok(())
}
